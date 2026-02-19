use goldfish::MemoryCortex;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub cortex: Arc<MemoryCortex>,
}
