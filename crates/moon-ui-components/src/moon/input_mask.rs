use gpui::SharedString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoonInputMaskToken {
    Digit,
    Letter,
    LetterOrDigit,
    Any,
    Separator(char),
}

impl MoonInputMaskToken {
    pub fn is_match(&self, ch: char) -> bool {
        match self {
            Self::Digit => ch.is_ascii_digit(),
            Self::Letter => ch.is_ascii_alphabetic(),
            Self::LetterOrDigit => ch.is_ascii_alphanumeric(),
            Self::Any => true,
            Self::Separator(expected) => *expected == ch,
        }
    }

    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator(_))
    }

    pub fn placeholder(&self) -> char {
        match self {
            Self::Separator(ch) => *ch,
            _ => '_',
        }
    }

    fn mask_char(&self, ch: char) -> char {
        match self {
            Self::Separator(ch) => *ch,
            _ => ch,
        }
    }

    fn unmask_char(&self, ch: char) -> Option<char> {
        match self {
            Self::Separator(_) => None,
            _ => Some(ch),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MoonInputMaskPattern {
    #[default]
    None,
    Pattern {
        pattern: SharedString,
        tokens: Vec<MoonInputMaskToken>,
    },
    Number {
        separator: Option<char>,
        fraction: Option<usize>,
    },
}

impl From<&str> for MoonInputMaskPattern {
    fn from(pattern: &str) -> Self {
        Self::new(pattern)
    }
}

impl From<String> for MoonInputMaskPattern {
    fn from(pattern: String) -> Self {
        Self::new(&pattern)
    }
}

impl From<MoonInputMaskPattern> for crate::input::MaskPattern {
    fn from(pattern: MoonInputMaskPattern) -> Self {
        match pattern {
            MoonInputMaskPattern::None => Self::None,
            MoonInputMaskPattern::Pattern { pattern, .. } => Self::new(pattern.as_ref()),
            MoonInputMaskPattern::Number {
                separator,
                fraction,
            } => Self::Number {
                separator,
                fraction,
            },
        }
    }
}

impl MoonInputMaskPattern {
    pub fn new(pattern: &str) -> Self {
        let tokens = pattern
            .chars()
            .map(|ch| match ch {
                '9' => MoonInputMaskToken::Digit,
                'A' => MoonInputMaskToken::Letter,
                '#' => MoonInputMaskToken::LetterOrDigit,
                '*' => MoonInputMaskToken::Any,
                _ => MoonInputMaskToken::Separator(ch),
            })
            .collect();

        Self::Pattern {
            pattern: SharedString::from(pattern),
            tokens,
        }
    }

    pub fn number(separator: Option<char>) -> Self {
        Self::Number {
            separator,
            fraction: None,
        }
    }

    pub fn number_with_fraction(separator: Option<char>, fraction: Option<usize>) -> Self {
        Self::Number {
            separator,
            fraction,
        }
    }

    pub fn placeholder(&self) -> Option<SharedString> {
        match self {
            Self::Pattern { tokens, .. } => Some(
                tokens
                    .iter()
                    .map(MoonInputMaskToken::placeholder)
                    .collect::<String>()
                    .into(),
            ),
            Self::Number { .. } | Self::None => None,
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            Self::Pattern { tokens, .. } => tokens.is_empty(),
            Self::Number { .. } => false,
        }
    }

    pub fn is_valid(&self, masked_text: &str) -> bool {
        if self.is_none() {
            return true;
        }

        match self {
            Self::Pattern { tokens, .. } => {
                let mut text_index = 0;
                let chars: Vec<char> = masked_text.chars().collect();
                for token in tokens {
                    if text_index >= chars.len() {
                        break;
                    }
                    if token.is_match(chars[text_index]) {
                        text_index += 1;
                    }
                }
                text_index == chars.len()
            }
            Self::Number { separator, .. } => is_valid_number_mask(masked_text, *separator),
            Self::None => true,
        }
    }

    pub fn is_valid_at(&self, ch: char, pos: usize) -> bool {
        if self.is_none() {
            return true;
        }

        match self {
            Self::Pattern { tokens, .. } => {
                if let Some(token) = tokens.get(pos) {
                    if token.is_match(ch) {
                        return true;
                    }
                    if token.is_separator()
                        && let Some(next_token) = tokens.get(pos + 1)
                    {
                        return next_token.is_match(ch);
                    }
                }
                false
            }
            Self::Number { .. } | Self::None => true,
        }
    }

    pub fn mask(&self, text: &str) -> SharedString {
        if self.is_none() {
            return SharedString::from(text);
        }

        match self {
            Self::Pattern { tokens, .. } => self.mask_pattern(text, tokens),
            Self::Number {
                separator,
                fraction,
            } => self.mask_number(text, *separator, *fraction),
            Self::None => SharedString::from(text),
        }
    }

    pub fn unmask(&self, masked_text: &str) -> String {
        match self {
            Self::Pattern { tokens, .. } => {
                let mut result = String::new();
                let chars: Vec<char> = masked_text.chars().collect();
                for (index, token) in tokens.iter().enumerate() {
                    if let Some(ch) = chars.get(index).copied()
                        && let Some(unmasked) = token.unmask_char(ch)
                    {
                        result.push(unmasked);
                    }
                }
                result
            }
            Self::Number { separator, .. } => {
                if let Some(separator) = *separator {
                    let mut result: String =
                        masked_text.chars().filter(|ch| *ch != separator).collect();
                    if result.contains('.') {
                        result = result.trim_end_matches('0').to_string();
                    }
                    result
                } else {
                    masked_text.to_owned()
                }
            }
            Self::None => masked_text.to_owned(),
        }
    }

    fn mask_pattern(&self, text: &str, tokens: &[MoonInputMaskToken]) -> SharedString {
        let mut result = String::new();
        let mut text_index = 0;
        let chars: Vec<char> = text.chars().collect();

        for (pos, token) in tokens.iter().enumerate() {
            if text_index >= chars.len() {
                break;
            }

            let ch = chars[text_index];
            if !token.is_separator() && !self.is_valid_at(ch, pos) {
                break;
            }

            let masked = token.mask_char(ch);
            result.push(masked);
            if ch == masked {
                text_index += 1;
            }
        }

        SharedString::from(result)
    }

    fn mask_number(
        &self,
        text: &str,
        separator: Option<char>,
        fraction: Option<usize>,
    ) -> SharedString {
        let Some(separator) = separator else {
            return SharedString::from(text);
        };

        let text = text.replace(separator, "");
        let mut parts = text.split('.');
        let int_part = parts.next().unwrap_or("");
        let frac_part = parts.next().map(|part| {
            part.chars()
                .take(fraction.unwrap_or(usize::MAX))
                .collect::<String>()
        });

        let mut chars: Vec<char> = int_part.chars().rev().collect();
        let sign = chars.iter().position(is_sign).map(|pos| chars.remove(pos));

        let mut grouped_reversed = String::new();
        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                grouped_reversed.push(separator);
            }
            grouped_reversed.push(*ch);
        }

        let mut result: String = grouped_reversed.chars().rev().collect();
        if let Some(frac) = frac_part
            && fraction != Some(0)
        {
            result.push('.');
            result.push_str(&frac);
        }
        if let Some(sign) = sign {
            result.insert(0, sign);
        }

        SharedString::from(result)
    }
}

fn is_valid_number_mask(masked_text: &str, separator: Option<char>) -> bool {
    if masked_text.is_empty() {
        return true;
    }

    let mut parts = masked_text.split('.');
    let int_part = parts.next().unwrap_or("");
    let frac_part = parts.next();
    if parts.next().is_some() {
        return false;
    }

    if int_part.is_empty() {
        return false;
    }

    let mut sign_count = 0;
    for (i, ch) in int_part.chars().enumerate() {
        if is_sign(&ch) {
            sign_count += 1;
            if sign_count > 1 || i != 0 {
                return false;
            }
            continue;
        }

        if ch.is_ascii_digit() || Some(ch) == separator {
            continue;
        }

        return false;
    }

    if let Some(frac) = frac_part
        && !frac
            .chars()
            .all(|ch| ch.is_ascii_digit() || Some(ch) == separator)
    {
        return false;
    }

    true
}

#[inline]
fn is_sign(ch: &char) -> bool {
    matches!(ch, '+' | '-')
}
