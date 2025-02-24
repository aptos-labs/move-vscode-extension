use crate::global_state::GlobalStateSnapshot;
use crate::line_index::{LineIndex, PositionEncoding};
use anyhow::format_err;
use camino::Utf8PathBuf;
use lang::files::FilePosition;
use line_index::{LineCol, TextRange, TextSize, WideLineCol};
use vfs::{AbsPathBuf, FileId};

pub(crate) fn abs_path(url: &lsp_types::Url) -> anyhow::Result<AbsPathBuf> {
    let path = url
        .to_file_path()
        .map_err(|()| format_err!("url is not a file"))?;
    Ok(AbsPathBuf::try_from(Utf8PathBuf::from_path_buf(path).unwrap()).unwrap())
}

pub(crate) fn vfs_path(url: &lsp_types::Url) -> anyhow::Result<vfs::VfsPath> {
    abs_path(url).map(vfs::VfsPath::from)
}

pub(crate) fn offset(line_index: &LineIndex, position: lsp_types::Position) -> anyhow::Result<TextSize> {
    let line_col = match line_index.encoding {
        PositionEncoding::Utf8 => LineCol {
            line: position.line,
            col: position.character,
        },
        PositionEncoding::Wide(enc) => {
            let line_col = WideLineCol {
                line: position.line,
                col: position.character,
            };
            line_index
                .index
                .to_utf8(enc, line_col)
                .ok_or_else(|| format_err!("Invalid wide col offset"))?
        }
    };
    let line_range = line_index.index.line(line_col.line).ok_or_else(|| {
        format_err!(
            "Invalid offset {line_col:?} (line index length: {:?})",
            line_index.index.len()
        )
    })?;
    let col = TextSize::from(line_col.col);
    let clamped_len = col.min(line_range.len());
    if clamped_len < col {
        tracing::error!(
            "Position {line_col:?} column exceeds line length {}, clamping it",
            u32::from(line_range.len()),
        );
    }
    Ok(line_range.start() + clamped_len)
}

pub(crate) fn text_range(line_index: &LineIndex, range: lsp_types::Range) -> anyhow::Result<TextRange> {
    let start = offset(line_index, range.start)?;
    let end = offset(line_index, range.end)?;
    match end < start {
        true => Err(format_err!("Invalid Range")),
        false => Ok(TextRange::new(start, end)),
    }
}

pub(crate) fn file_id(snap: &GlobalStateSnapshot, url: &lsp_types::Url) -> anyhow::Result<FileId> {
    snap.url_to_file_id(url)
}

pub(crate) fn file_position(
    snap: &GlobalStateSnapshot,
    tdpp: lsp_types::TextDocumentPositionParams,
) -> anyhow::Result<FilePosition> {
    let file_id = file_id(snap, &tdpp.text_document.uri)?;
    let line_index = snap.file_line_index(file_id)?;
    let offset = offset(&line_index, tdpp.position)?;
    Ok(FilePosition { file_id, offset })
}
