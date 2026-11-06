//! Named TOML table/key edits. File selection, authorization, locks and commit
//! belong to callers; no partial document is returned after an error.
use toml_edit::{DocumentMut, InlineTable, Item, Table, TableLike};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchError {
    Parse,
    Shape,
    InvalidPath,
}

#[derive(Default)]
pub struct TomlPatch {
    /// Paths are literal key segments, including quoted keys containing dots.
    /// Explicit targets win over remove. Missing table ancestors are created.
    pub set: Vec<(Vec<String>, Item)>,
    pub remove: Vec<Vec<String>>,
    /// Existing values are kept, even when different from the seed.
    pub seed: Vec<(Vec<String>, Item)>,
}

impl TomlPatch {
    /// Prepare bytes in a private document; missing files start empty.
    /// Invalid UTF-8/TOML and non-table ancestors fail with payload-free errors.
    pub fn apply(&self, pre: Option<&[u8]>) -> Result<Vec<u8>, PatchError> {
        let mut doc = parse(pre.unwrap_or_default())?;
        for path in self
            .remove
            .iter()
            .chain(self.set.iter().map(|(p, _)| p))
            .chain(self.seed.iter().map(|(p, _)| p))
        {
            if path.is_empty() || path.len() > 64 {
                return Err(PatchError::InvalidPath);
            }
        }
        for path in &self.remove {
            if !self.set.iter().any(|(target, _)| target == path) {
                edit(doc.as_table_mut(), path, None, false, false)?;
            }
        }
        for (path, item) in &self.set {
            edit(doc.as_table_mut(), path, Some(item), false, false)?;
        }
        for (path, item) in &self.seed {
            edit(doc.as_table_mut(), path, Some(item), true, false)?;
        }
        let output = doc.to_string().into_bytes();
        parse(&output)?;
        Ok(output)
    }
}

fn parse(bytes: &[u8]) -> Result<DocumentMut, PatchError> {
    if bytes.len() > 1024 * 1024 {
        return Err(PatchError::Parse);
    }
    std::str::from_utf8(bytes)
        .map_err(|_| PatchError::Parse)?
        .parse()
        .map_err(|_| PatchError::Parse)
}

fn edit(
    table: &mut dyn TableLike,
    path: &[String],
    item: Option<&Item>,
    seed: bool,
    inline: bool,
) -> Result<(), PatchError> {
    let (key, rest) = path.split_first().ok_or(PatchError::InvalidPath)?;
    if !rest.is_empty() {
        if !table.contains_key(key) {
            if item.is_none() {
                return Ok(());
            }
            let child = if inline {
                Item::Value(InlineTable::new().into())
            } else {
                let mut child = Table::new();
                child.set_implicit(true);
                Item::Table(child)
            };
            table.insert(key, child);
        }
        let child = table.get_mut(key).ok_or(PatchError::Shape)?;
        let child_inline = child.is_inline_table();
        return edit(
            child.as_table_like_mut().ok_or(PatchError::Shape)?,
            rest,
            item,
            seed,
            child_inline,
        );
    }
    let Some(item) = item else {
        table.remove(key);
        return Ok(());
    };
    if seed && table.contains_key(key) {
        return Ok(());
    }
    if item.is_none() {
        return Err(PatchError::Shape);
    }
    let mut replacement = if inline {
        Item::Value(item.clone().into_value().map_err(|_| PatchError::Shape)?)
    } else {
        item.clone()
    };
    match table.get_mut(key) {
        Some(slot) => {
            match (&*slot, &mut replacement) {
                (Item::Value(old), Item::Value(new)) => *new.decor_mut() = old.decor().clone(),
                (Item::Table(old), Item::Table(new)) => {
                    *new.decor_mut() = old.decor().clone();
                    if let Some(position) = old.position() {
                        new.set_position(position);
                    }
                }
                _ => {}
            }
            *slot = replacement;
        }
        None => {
            table.insert(key, replacement);
        }
    }
    Ok(())
}
