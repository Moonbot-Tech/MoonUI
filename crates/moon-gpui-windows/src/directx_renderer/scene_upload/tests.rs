use super::*;

/// Drive one frame the way the renderer does: upload or skip.
fn frame(tracker: &mut SceneUploadTracker, revision: u64) {
    if tracker.should_upload(revision) {
        tracker.mark_uploaded(revision);
    } else {
        tracker.record_skip();
    }
}

/// Catches `scene_upload.rs:should_upload` always answering true: every GPU-only frame over
/// an unchanged scene would re-upload all six scene buffers.
#[test]
fn gpu_only_frames_do_not_reupload() {
    let mut tracker = SceneUploadTracker::default();
    for _ in 0..10 {
        frame(&mut tracker, 7);
    }
    assert_eq!(tracker.uploads(), 1);
    assert_eq!(tracker.skips(), 9);
}

/// Catches a tracker that never uploads again: a changed scene would draw stale buffers.
#[test]
fn new_revision_uploads() {
    let mut tracker = SceneUploadTracker::default();
    frame(&mut tracker, 7);
    assert!(tracker.should_upload(8));
}

/// Catches revision 0 (unknown scene) being trusted as uploaded.
#[test]
fn revision_zero_always_uploads() {
    let mut tracker = SceneUploadTracker::default();
    tracker.mark_uploaded(0);
    assert!(tracker.should_upload(0));
}

/// Catches `invalidate` not forgetting the revision: after a device loss the new buffers
/// would stay empty until the scene changed.
#[test]
fn invalidate_forces_upload() {
    let mut tracker = SceneUploadTracker::default();
    frame(&mut tracker, 7);
    tracker.invalidate();
    assert!(tracker.should_upload(7));
}
