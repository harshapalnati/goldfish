use std::sync::Arc;
use goldfish::MemoryCortex;

#[derive(Clone)]
pub struct AppState {
    pub cortex: Arc<MemoryCortex>,
}
