//! Tar archives — `bundle pack` writes `.tgz`; the http/fs sources
//! unpack tarballs and `git archive` output here. Paths sanitize on
//! unpack (no absolute/`..` escapes), modes keep the executable bit.

use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use tar::{Archive, Builder, EntryType};

use crate::Error;
use crate::digest::payload_files;

/// `dir` → a gzipped tarball at `out` — payload files at the archive
/// root, sorted for deterministic entry order.
pub fn pack_tgz(src_dir: &Path, out: &Path) -> Result<PathBuf, Error> {
    let file = fs::File::create(out).map_err(Error::io(format!("create {}", out.display())))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = Builder::new(encoder);
    for rel in payload_files(src_dir)? {
        builder
            .append_path_with_name(src_dir.join(&rel), &rel)
            .map_err(Error::io(format!("archive {}", rel.display())))?;
    }
    let encoder = builder.into_inner().map_err(Error::io("finish tar"))?;
    encoder.finish().map_err(Error::io("finish gzip"))?;
    Ok(out.to_path_buf())
}

/// Unpack a plain `.tar` stream (`git archive --format=tar` output).
pub fn untar(reader: impl Read, dest: &Path) -> Result<(), Error> {
    unpack(Archive::new(reader), dest)
}

/// Unpack a gzipped tarball stream (`.tgz`/`.tar.gz`).
pub fn untar_gz(reader: impl Read, dest: &Path) -> Result<(), Error> {
    unpack(Archive::new(GzDecoder::new(reader)), dest)
}

fn unpack<R: Read>(mut archive: Archive<R>, dest: &Path) -> Result<(), Error> {
    fs::create_dir_all(dest).map_err(Error::io(format!("create {}", dest.display())))?;
    let entries = archive.entries().map_err(Error::io("read tar entries"))?;
    for entry in entries {
        let mut entry = entry.map_err(Error::io("read tar entry"))?;
        let path = entry.path().map_err(Error::io("read tar entry path"))?;
        let Some(rel) = sanitize(&path)? else {
            continue;
        };
        let target = dest.join(&rel);
        match entry.header().entry_type() {
            EntryType::Directory => {
                fs::create_dir_all(&target)
                    .map_err(Error::io(format!("create {}", target.display())))?;
            }
            EntryType::Regular => {
                let parent = target.parent().unwrap_or(dest);
                fs::create_dir_all(parent)
                    .map_err(Error::io(format!("create {}", parent.display())))?;
                let mut file = fs::File::create(&target)
                    .map_err(Error::io(format!("create {}", target.display())))?;
                io::copy(&mut entry, &mut file)
                    .map_err(Error::io(format!("write {}", target.display())))?;
                set_mode(&entry, &target)?;
            }
            EntryType::Symlink => {
                write_symlink(&entry, &target)?;
            }
            other => {
                return Err(Error::Failed(format!(
                    "unsupported tar entry type {other:?} at {}",
                    rel.display()
                )));
            }
        }
    }
    Ok(())
}

/// `./`-stripping + tar-slip rejection: absolute paths and `..`
/// components are a hard error, root entries skip.
fn sanitize(path: &Path) -> Result<Option<PathBuf>, Error> {
    let mut rel = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => rel.push(part),
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(Error::Failed(format!(
                    "unsafe path in archive: {}",
                    path.display()
                )));
            }
        }
    }
    if rel.as_os_str().is_empty() {
        Ok(None)
    } else {
        Ok(Some(rel))
    }
}

#[cfg(unix)]
fn set_mode<R: Read>(entry: &tar::Entry<'_, R>, target: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    let mode = entry.header().mode().map_err(Error::io("read tar mode"))?;
    fs::set_permissions(target, fs::Permissions::from_mode(mode))
        .map_err(Error::io(format!("chmod {}", target.display())))
}

#[cfg(unix)]
fn write_symlink<R: Read>(entry: &tar::Entry<'_, R>, target: &Path) -> Result<(), Error> {
    let link = entry
        .link_name()
        .map_err(Error::io("read tar symlink"))?
        .ok_or_else(|| Error::Failed("symlink entry without a target".to_string()))?;
    // Relative links only — an absolute or escaping link is a slip.
    if link.is_absolute() || link.components().any(|c| c == Component::ParentDir) {
        return Err(Error::Failed(format!(
            "unsafe symlink in archive: {}",
            link.display()
        )));
    }
    let parent = target.parent().unwrap_or_else(|| Path::new("/"));
    fs::create_dir_all(parent).map_err(Error::io(format!("create {}", parent.display())))?;
    std::os::unix::fs::symlink(&link, target)
        .map_err(Error::io(format!("symlink {}", target.display())))
}

#[cfg(not(unix))]
fn set_mode<R: Read>(_entry: &tar::Entry<'_, R>, _target: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(not(unix))]
fn write_symlink<R: Read>(_entry: &tar::Entry<'_, R>, target: &Path) -> Result<(), Error> {
    Err(Error::Failed(format!(
        "symlinks unsupported on this platform: {}",
        target.display()
    )))
}
