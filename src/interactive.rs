//! Interactive selection UI: commit picker and message editor.

use crate::splitter::Group;
use anyhow::Result;
use colored::Colorize;

/// Result of the multi-select commit picker.
#[derive(Debug)]
pub enum SelectResult {
    /// User explicitly confirmed some selections.
    Some(Vec<usize>),
    /// User confirmed with nothing selected (explicit empty).
    None,
    /// User cancelled via Esc/Ctrl+C.
    Cancelled,
}

/// Let the user pick which groups to commit via a multi-select menu.
pub fn select_commits(groups: &[Group], msgs: &[String]) -> SelectResult {
    let items: Vec<String> = groups
        .iter()
        .zip(msgs.iter())
        .map(|(g, m)| {
            format!(
                "📦 {:<15} 💬 {:<45} ({:>2} files, {}{})",
                g.name.bold().cyan(),
                m,
                g.file_count,
                format!("+{}", g.insertions).green(),
                format!(" -{}", g.deletions).red()
            )
        })
        .collect();

    let selection = dialoguer::MultiSelect::new()
        .with_prompt("Select atomic commits to apply [space: toggle, enter: confirm, esc: cancel]")
        .items(&items)
        .interact_opt();

    match selection {
        Ok(Some(sel)) if sel.is_empty() => SelectResult::None,
        Ok(Some(sel)) => SelectResult::Some(sel),
        Ok(None) | Err(_) => SelectResult::Cancelled,
    }
}

/// Let the user edit a commit message before committing.
pub fn edit_message(msg: &str) -> Result<String> {
    let edited: String = dialoguer::Input::new()
        .with_prompt("✍️ Edit commit message")
        .with_initial_text(msg)
        .interact_text()?;
    Ok(edited)
}
