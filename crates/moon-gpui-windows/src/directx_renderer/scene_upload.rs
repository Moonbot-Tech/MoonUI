/// Remembers which scene revision the six scene pipelines already hold, so a
/// frame that only re-presents GPU canvases over an unchanged scene skips the
/// buffer upload.
#[derive(Default)]
pub(crate) struct SceneUploadTracker {
    uploaded: Option<u64>,
    uploads: u64,
    skips: u64,
}

impl SceneUploadTracker {
    /// Whether the scene with `revision` must be uploaded; revision 0 is unknown
    /// and always uploads.
    pub(crate) fn should_upload(&mut self, revision: u64) -> bool {
        if revision == 0 || self.uploaded != Some(revision) {
            return true;
        }
        self.skips += 1;
        false
    }

    pub(crate) fn mark_uploaded(&mut self, revision: u64) {
        self.uploaded = Some(revision);
        self.uploads += 1;
    }

    /// Forgets the uploaded revision after a failed upload or a device loss.
    pub(crate) fn invalidate(&mut self) {
        self.uploaded = None;
    }

    #[allow(dead_code)]
    pub(crate) fn uploads(&self) -> u64 {
        self.uploads
    }

    #[allow(dead_code)]
    pub(crate) fn skips(&self) -> u64 {
        self.skips
    }
}
