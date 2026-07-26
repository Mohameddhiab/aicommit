//! Interactive selection UI: commit picker and message editor.

use crate::splitter::Group;
use anyhow::Result;

/// Let the user pick which groups to commit via a multi-select menu.
/// Returns the indices (into `groups`) of the selected items.
pub fn select_commits(groups: &[Group], msgs: &[String]) -> Vec<usize> {
    let items: Vec<String> = groups
        .iter()
        .zip(msgs.iter())
        .map(|(g, m)| format!("[{}]  {m}", g.name))
        .collect();

    let selection = dialoguer::MultiSelect::new()
        .with_prompt("Select commits to apply (space to toggle, enter to confirm)")
        .items(&items)
        .interact()
        .ok()
        .unwrap_or_default();

    // If nothing selected, the user may have hit enter by accident — default to all.
    if selection.is_empty() {
        return (0..groups.len()).collect();
    }
    selection
}

/// Let the user edit a commit message before committing.
pub fn edit_message(msg: &str) -> Result<String> {
    let edited: String = dialoguer::Input::new()
        .with_prompt("Commit message")
        .with_initial_text(msg)
        .interact_text()?;
    Ok(edited)
}
