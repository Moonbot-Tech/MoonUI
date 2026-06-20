# GPU Canvas Text API

Дата: 2026-06-20.

Цель: дать `gpu_canvas` нормальный способ рисовать single-line UTF-8 подписи
внутри GPU-контрола без `cx.notify()`, без top-down перерисовки GPUI дерева и
без отдельного самодельного текстового движка в приложении.

## Что решаем

Есть классы контролов, где почти вся картинка живет в собственном GPU-pass:
графики, игровые viewport-ы, кастомные курсоры, видео/осциллографы, CAD/сцены.
Им нужно на каждом frame tick самостоятельно решить:

1. изменились ли данные или камера;
2. надо ли показывать этот кадр;
3. какой текст нарисовать поверх/под своим GPU-pass.

Текст при этом должен использовать тот же shaping, glyph atlas, emoji/color
glyph path, subpixel режим, clip/content mask и platform renderer, что обычный
GPUI text. Это не полный layout/styling stack GPUI: multiline wrap,
background, underline и strikethrough остаются задачей обычного GPUI text.
Приложение не должно вручную лезть в DirectWrite/CoreText, Metal, DX11 или wgpu
ради одной подписи на графике.

## Публичная форма API

Минимальная модель:

```rust
impl GpuCanvasDriver for ChartDriver {
    fn frame(&mut self, info: GpuFrameInfo) -> GpuFrameDecision {
        if self.chart_should_move_or_data_changed(info) {
            GpuFrameDecision::RequestPresent
        } else {
            GpuFrameDecision::Skip
        }
    }

    fn prepare_text(&mut self, ctx: &mut GpuCanvasTextContext<'_>) -> anyhow::Result<()> {
        self.price_axis_label.draw(
            ctx,
            point(px(8.), px(32.)),
            "63120.5",
            self.font.clone(),
            px(12.),
            px(16.),
            self.theme.axis_text,
        )?;
        Ok(())
    }
}
```

`frame()` принимает решение о кадре. `prepare_text()` только описывает текст,
который должен попасть в тот же реально рисуемый GPU frame.

## Два режима текста

### `GpuCanvasTextContext::draw_text`

Одноразовый удобный путь:

```rust
ctx.draw_text(origin, text, font, font_size, line_height, color)?;
```

Он shape-ит строку при каждом вызове. Это нормально для холодных подписей,
редкого debug text или небольших списков, которые не меняются на каждом tick.

### `GpuCanvasTextRun`

Основной горячий путь:

```rust
struct ChartDriver {
    price_axis_labels: Vec<GpuCanvasTextRun>,
    cursor_price: GpuCanvasTextRun,
    cursor_time: GpuCanvasTextRun,
}
```

`GpuCanvasTextRun` хранится рядом с логической подписью. Его надо вызывать
каждый frame, но он повторно shape-ит строку только при изменении ключа:

```text
(text, font, font_size)
```

Движение по экрану, цвет, `line_height`, anchor и слой не инвалидируют shape.
Это принципиально: если график сдвинулся по X, а надписи цены по Y не
изменились, мы снова отправляем glyph sprites на текущий кадр, но не делаем
дорогой shaping заново.

Пример:

```rust
let label = if btc_price < 99000.0 { "LOW" } else { "HIGH" };
self.center_label.draw_aligned(
    ctx,
    chart_center,
    label,
    self.font.clone(),
    px(18.),
    px(24.),
    self.theme.warning_text,
    0.5,
    0.5,
)?;
```

Если `label` не менялся 2400 кадров, shape будет один раз, draw будет 2400
раз. Если цена пересекла порог и строка поменялась, shape произойдет один раз
на новой строке.

## Много строк

Если у графика есть сотни или тысячи разных подписей, не надо вручную
проверять каждую строку на изменение перед draw. Правильная модель:

```text
one logical label -> one retained GpuCanvasTextRun
```

На каждом реально рисуемом кадре приложение вызывает `draw`/`draw_aligned` для
всех видимых подписей. `GpuCanvasTextRun` сам сравнивает shape-key
`(text, font, font_size)`:

- строка не изменилась — shaping не делается, только эмитятся glyph sprites для
  текущего кадра;
- изменилась позиция, цвет, anchor или line height — shaping не делается;
- изменилась сама строка, font или font size — reshaping происходит только для
  этой logical label.

Иными словами, не нужен внешний `HashMap<String, ShapedLine>` по тексту. Такой
map ошибочен: одинаковый текст у разных логических подписей может жить в разных
местах, с разным lifecycle и разным смыслом. Cache должен принадлежать
логической подписи, а не строковому значению.

Hot path принимает borrowed text (`&str`, `&String`, `&SharedString`, etc.) и
создаёт owned `SharedString` только когда shape-key действительно изменился.
Передача `text.as_str()` в каждый present-frame не должна копировать строку на
неизменившейся подписи.

### MoonProto chart text

MoonProto сейчас отдаёт chart text как full replacement snapshot:

```rust
ChartTextSnapshot {
    market_name: String,
    filter_lines: Vec<String>,
    debug_lines: Vec<String>,
}
```

У `filter_lines` и `debug_lines` нет per-line stable id. Поэтому правильный
cache-key для этих строк — индекс строки внутри последнего snapshot-а:

```rust
struct ChartDriver {
    filter_lines: Vec<String>,
    filter_runs: Vec<GpuCanvasTextRun>,
    debug_lines: Vec<String>,
    debug_runs: Vec<GpuCanvasTextRun>,
}
```

При новом `ChartTextSnapshot`:

```rust
self.filter_lines = snapshot.filter_lines;
self
    .filter_runs
    .resize_with(self.filter_lines.len(), GpuCanvasTextRun::default);
self.filter_runs.truncate(self.filter_lines.len());
```

В `prepare_text()`:

```rust
for (i, text) in self.filter_lines.iter().enumerate() {
    let origin = point(px(12.0), px(18.0 + i as f32 * 14.0));
    self.filter_runs[i].draw(
        ctx,
        origin,
        text.as_str(),
        self.font.clone(),
        px(11.0),
        px(14.0),
        self.theme.filter_text,
    )?;
}
```

Это означает: все видимые строки заново отправляются как glyph sprites в каждый
present-frame, потому что frame content новый. Но unchanged строки не
shape-ятся заново. Если из тысячи строк поменялась одна, expensive shaping
будет только у неё.

`HashMap<StableId, GpuCanvasTextRun>` нужен для других типов chart labels, где
есть стабильный id: ордера, alert objects, zones, markers, user levels. Там
ключом должен быть `OrderId`, `ObjUid`, `MarkerId` и т.п., а не текст.

## Слои

У `GpuCanvas` есть два независимых слоя:

```rust
gpu_canvas(driver)
    .under()       // native canvas layer
    .text_over();  // text layer
```

`canvas_layer` отвечает, где рисуется сам native pass. `text_layer` отвечает,
где композится текст, созданный через `prepare_text()`.

Зачем разделять:

- график может рисоваться под обычными GPUI панелями, но readout/cursor text
  должен быть поверх;
- debug overlay может быть поверх всего canvas;
- фоновые подписи/watermark могут быть под обычной сценой.

Если пользователь вызывает `.over()`, по умолчанию и canvas, и text становятся
`OverScene`. При необходимости text layer можно переопределить явно:

```rust
gpu_canvas(driver).over().text_under();
```

## Где это используется в терминале

Все, что визуально находится в зоне графика, должно идти через этот механизм:

- cursor crosshair readout;
- цена и время под курсором;
- подписи сетки цены/времени;
- подписи ордеров, зон, markers, пользовательских уровней;
- debug/perf overlay внутри chart viewport.

Не надо возвращать эти подписи в GPUI overlay через `cx.notify()`. Иначе
mousemove снова начнет будить весь компонент/окно ради одной цифры.

## Важные инварианты

1. Решение о кадре принимает `GpuCanvasDriver::frame()`.
2. `prepare_text()` вызывается только для кадра, который будет реально
   показан.
3. Present у окна общий: если один canvas в окне попросил present, text frame
   готовится для всех видимых canvas-ов этого окна, чтобы соседние canvas-ы не
   теряли подписи.
4. Default clip для text sprites равен `canvas.bounds ∩ content_mask`, как и
   scissor native canvas pass-а. Текст не должен вылезать за прямоугольник
   своего `gpu_canvas`.
5. `prepare_text()` пишет во временный frame одного canvas-а. Если оно вернуло
   ошибку, частично набитые glyph sprites отбрасываются целиком.
6. `prepare_text()` не должен менять GPUI state, вызывать `cx.notify()` или
   строить обычные views.
7. Текстовые handles живут в driver state. Не создавать их заново в каждом
   `prepare_text()`.
8. Shape cache ключуется по смыслу glyph layout: строка, font, font size.
9. Position/color/anchor не являются причиной reshaping.
10. Glyph sprites очищаются каждый frame, потому что frame content новый.
   Shaped text cache не очищается, потому что это retained CPU cache.

## Как устроено внутри

Основные точки кода:

```text
crates/moon-gpui/src/gpu_canvas.rs
crates/moon-gpui/src/scene.rs
crates/moon-gpui/src/window.rs
crates/moon-gpui-windows/src/directx_renderer.rs
crates/moon-gpui-macos/src/metal_renderer.rs
crates/moon-gpui-wgpu/src/wgpu_renderer.rs
```

`gpu_canvas.rs` объявляет:

- `GpuCanvasDriver::prepare_text`;
- `GpuCanvasTextContext`;
- `GpuCanvasTextRun`;
- `GpuCanvasTextFrame`;
- `GpuCanvas::text_under/text_over/text_layer`.

`scene.rs` хранит два text frame-а:

- `gpu_canvas_text_under_scene`;
- `gpu_canvas_text_over_scene`.

`window.rs`:

1. вызывает `driver.frame(info)`;
2. понимает, будет ли этот tick реально представлен;
3. если будет, вызывает `driver.prepare_text(ctx)` для всех canvas-ов окна;
4. даёт `prepare_text(ctx)` доступ к `ctx.bounds()`, `ctx.scale_factor()`,
   `ctx.content_mask()`, `ctx.canvas_layer()` и `ctx.text_layer()`;
5. кладет glyph sprites в `text_layer`, а не обязательно в слой самого canvas.

Платформенные renderers рисуют text frame теми же sprite paths, что обычный
GPUI text.

Text z-order сейчас layer-level, не per-canvas interleave:

```text
all under gpu canvases
all under gpu canvas text
ordinary GPUI scene
all over gpu canvases
all over gpu canvas text
```

Это сделано осознанно из-за независимого `text_layer`: canvas может быть under,
а его text overlay — over. Если будущему API понадобится строгий порядок
`canvas A -> text A -> canvas B -> text B` для перекрывающихся canvas-ов, это
надо делать отдельным per-canvas text-frame расширением, а не незаметно менять
семантику текущего layer-level overlay.

## Обновление базы Zed/GPUI

При подтягивании новой upstream базы нельзя просто проверить, что проект
компилится. Надо пройти этот короткий чеклист.

### 1. `Window` frame loop

Проверить, что `frame_gpu_canvases` все еще:

- вызывается на platform frame tick;
- вызывает `GpuCanvasDriver::frame()` до present;
- не dirty-ит GPUI view tree;
- вызывает `prepare_text()` только когда кадр будет показан;
- если один canvas попросил present, вызывает `prepare_text()` для всех canvas-ов
  окна;
- передает в `GpuCanvasTextContext` bounds, effective clip и оба слоя: canvas
  layer и text layer;
- append-ит text sprites в общий frame только после успешного `prepare_text()`.

Если upstream поменял `on_request_frame`, `needs_present`, `force_render`,
`present_if_needed` или device recovery path, этот блок надо перечитать руками.
Главный смысл не должен поменяться: canvas решает, а текст готовится в тот же
кадр, без задержки на следующий tick.

### 2. Scene replay/cache

Проверить `Scene::insert_gpu_canvas` и `Scene::replay`.

Canvas paint operation должен сохранять `PaintGpuCanvas` вместе с `text_layer`.
При replay старой сцены text layer не должен теряться, иначе UI dirty reuse
вернет canvas, но текст внезапно окажется в другом z-order.

### 3. Glyph atlas и stale texture ids

После device-lost или atlas reset старые sprite texture id могут устареть.
Renderers должны пропускать stale sprite batch/texture lookup, а не panic-ить.

Особенно проверить:

```text
DirectX atlas get_texture_view path
Metal texture lookup path
wgpu texture lookup path
```

Поведение при ошибке: пропущенный batch/frame, не падение процесса.

### 4. Renderer order

Порядок должен остаться таким:

```text
under gpu canvas
under gpu canvas text
ordinary GPUI scene
over gpu canvas
over gpu canvas text
```

Если upstream поменял batching/render order, не переносить text frame в обычные
primitive batches автоматически. Это отдельный retained слой, иначе вернется
проблема с top-down repaint.

### 5. Text system API

Проверить, что `WindowTextSystem::shape_line`, `TextRun`, `ShapedLine`,
`RenderGlyphParams`, `MonochromeSprite`, `SubpixelSprite`, `PolychromeSprite`
сохранили смысл.

Если upstream изменил shape/raster API, новый адаптер должен сохранить внешнее
правило:

```text
draw every frame, shape only when text/font/font_size changes
```

### 6. Терминальный chart path

После обновления базы проверить реальные сценарии:

- mousemove над графиком не вызывает `cx.notify()` ради cursor readout;
- cursor text двигается плавно вместе с crosshair;
- price/time axis labels не reshape-ятся каждый tick при простом X-scroll;
- 4 графика/стек графиков не будят Shell/Orders top-down;
- device-lost/recreate не падает из-за stale atlas sprites.

## Что не делать

- Не рисовать chart labels обычными GPUI elements поверх графика.
- Не заводить отдельный DirectX/Metal/wgpu text renderer в приложении.
- Не shape-ить строки в `frame()`: `frame()` только решает, нужен ли кадр.
- Не делать `prepare_text()` обязательным источником present. Present request
  идет из `frame()`, текст только наполняет кадр.
- Не смешивать glyph cache и frame sprite list: shape cache retained, sprite
  list per-frame.
