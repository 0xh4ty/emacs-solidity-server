use once_cell::sync::OnceCell;
use std::sync::Arc;

use crate::solc::manager::SolcManager;

pub static SOLC_MANAGER: OnceCell<Arc<SolcManager>> = OnceCell::new();
