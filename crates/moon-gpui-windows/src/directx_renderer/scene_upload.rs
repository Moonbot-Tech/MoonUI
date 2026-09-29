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
    pub(crate) fn should_upload(&self, revision: u64) -> bool {
        revision == 0 || self.uploaded != Some(revision)
    }

    /// Counts a frame that kept the already uploaded scene.
    pub(crate) fn record_skip(&mut self) {
        self.skips += 1;
    }

    pub(crate) fn mark_uploaded(&mut self, revision: u64) {
        self.uploaded = Some(revision);
        self.uploads += 1;
    }

    /// Forgets the uploaded revision after a failed upload or a device loss.
    pub(crate) fn invalidate(&mut self) {
        self.uploaded = None;
    }

    pub(crate) fn uploads(&self) -> u64 {
        self.uploads
    }

    pub(crate) fn skips(&self) -> u64 {
        self.skips
    }
}

#[cfg(test)]
mod tests;
