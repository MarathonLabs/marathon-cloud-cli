use std::path::Path;

use anyhow::Context;
use async_zip::{tokio::write::ZipFileWriter, Compression, ZipEntryBuilder};
use log::debug;
use tokio::{fs::File, io::AsyncReadExt};
use walkdir::DirEntry;

pub async fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &str,
    mut writer: T,
) -> anyhow::Result<()>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    let unix_permissions = 0o755;
    let compression_method = Compression::Deflate;
    let mut zip = ZipFileWriter::with_tokio(&mut writer);

    let prefix = Path::new(prefix);
    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(prefix)?;
        let path_as_string = name
            .to_str()
            .map(str::to_owned)
            .with_context(|| format!("{name:?} Is a Non UTF-8 Path"))?;

        if path.is_file() {
            debug!("adding file {path:?} as {name:?} ...");
            let mut f = File::open(path).await?;
            f.read_to_end(&mut buffer).await?;

            let builder = ZipEntryBuilder::new(path_as_string.into(), compression_method)
                .unix_permissions(unix_permissions);
            zip.write_entry_whole(builder, &buffer).await?;

            buffer.clear();
        }
    }
    zip.close().await?;
    Ok(())
}
