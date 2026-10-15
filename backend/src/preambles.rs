use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreambleBlock {
    pub id: String,
    pub text: String,
    pub enabled: bool,
}

fn config_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!(%error, "application preamble store failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "preamble_store_failed: inspect server diagnostics".into(),
    )
}

fn validate(blocks: &[PreambleBlock]) -> Result<(), (StatusCode, String)> {
    let mut ids = std::collections::HashSet::new();
    if blocks.len() > 16
        || blocks.iter().any(|block| {
            block.id.is_empty()
                || block.id.len() > 64
                || !block
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
                || block.text.trim().is_empty()
                || block.text.len() > 4096
                || !ids.insert(&block.id)
        })
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid_preambles: use up to16 unique ASCII ids and nonempty text up to4096 bytes"
                .into(),
        ));
    }
    Ok(())
}

pub(super) fn load(state: &AppState) -> Result<Vec<PreambleBlock>, (StatusCode, String)> {
    let path = state.store.root.join("preambles.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(config_error(error)),
    };
    let blocks: Vec<PreambleBlock> = serde_json::from_slice(&bytes).map_err(config_error)?;
    validate(&blocks).map_err(|_| config_error("invalid persisted configuration"))?;
    Ok(blocks)
}

pub(super) async fn get(
    State(state): State<AppState>,
) -> Result<Json<Vec<PreambleBlock>>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    Ok(Json(load(&state)?))
}

pub(super) async fn save(
    State(state): State<AppState>,
    Json(blocks): Json<Vec<PreambleBlock>>,
) -> Result<Json<Vec<PreambleBlock>>, (StatusCode, String)> {
    let _transaction = state.transaction.lock().await;
    validate(&blocks)?;
    std::fs::create_dir_all(&state.store.root).map_err(config_error)?;
    std::fs::write(
        state.store.root.join("preambles.json"),
        serde_json::to_vec(&blocks).map_err(config_error)?,
    )
    .map_err(config_error)?;
    Ok(Json(load(&state)?))
}

pub(super) fn assembled(blocks: &[PreambleBlock]) -> String {
    blocks
        .iter()
        .filter(|block| block.enabled)
        .map(|block| format!("[preamble:{}]\n{}", block.id, block.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}
