// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
use sandbox_data::{
    VirtualPath,
    wdf::{WdfDirectory, WdfHeader, path_id},
};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Assets {
    root: PathBuf,
    archives: Vec<(File, WdfDirectory)>,
    resolved: HashMap<String, PathBuf>,
}
impl Assets {
    pub fn open(root: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err("--assets must name an installation directory".into());
        }
        let mut assets = Self {
            root,
            archives: Vec::new(),
            resolved: HashMap::new(),
        };
        for name in ["data.wdf", "c3.wdf"] {
            if let Some(path) = assets.loose(name)? {
                let mut file = File::open(path)?;
                let length = file.metadata()?.len();
                let mut bytes = [0; 12];
                file.read_exact(&mut bytes)?;
                let header = WdfHeader::parse(&bytes)?;
                let size = header.directory_length()?;
                if size > 64 * 1024 * 1024 {
                    return Err("WDF directory exceeds safety limit".into());
                }
                file.seek(SeekFrom::Start(header.directory_offset as u64))?;
                let mut bytes = vec![0; size];
                file.read_exact(&mut bytes)?;
                assets
                    .archives
                    .push((file, WdfDirectory::parse(header, &bytes, length)?));
            }
        }
        Ok(assets)
    }
    fn loose(&mut self, name: &str) -> Result<Option<PathBuf>> {
        let virtual_path = VirtualPath::try_from(name)?;
        let name = virtual_path.as_str();
        if let Some(path) = self.resolved.get(name) {
            return Ok(Some(path.clone()));
        }
        let mut path = self.root.clone();
        for component in name.split('/') {
            let exact = path.join(component);
            path = if exact.exists() {
                exact
            } else {
                let Some(entry) = std::fs::read_dir(&path)?.filter_map(|e| e.ok()).find(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(component)
                }) else {
                    return Ok(None);
                };
                entry.path()
            };
            path = path.canonicalize()?;
            if !path.starts_with(&self.root) {
                return Err("asset symlink leaves installation".into());
            }
        }
        if !path.is_file() {
            return Ok(None);
        }
        self.resolved.insert(name.to_owned(), path.clone());
        Ok(Some(path))
    }
    pub fn read(&mut self, name: &str) -> Result<Vec<u8>> {
        let virtual_path = VirtualPath::try_from(name)?;
        if let Some(path) = self.loose(name)? {
            let file = File::open(path)?;
            if file.metadata()?.len() > 128 * 1024 * 1024 {
                return Err("asset exceeds 128 MiB safety limit".into());
            }
            let mut bytes = Vec::new();
            file.take(128 * 1024 * 1024).read_to_end(&mut bytes)?;
            return Ok(bytes);
        }
        let id = path_id(&virtual_path)?;
        for (file, directory) in &mut self.archives {
            if let Some(entry) = directory.get(id) {
                if entry.size > 128 * 1024 * 1024 {
                    return Err("archive entry exceeds safety limit".into());
                }
                file.seek(SeekFrom::Start(entry.offset as u64))?;
                let mut bytes = vec![0; entry.size as usize];
                file.read_exact(&mut bytes)?;
                return Ok(bytes);
            }
        }
        Err(format!("Missing installed asset: {name}").into())
    }
    pub fn image(&mut self, name: &str) -> Result<image::RgbaImage> {
        let bytes = self.read(name)?;

        let mut reader =
            image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
        if reader.format().is_none() {
            reader.set_format(image::ImageFormat::from_path(name)?);
        }
        Ok(reader.decode()?.into_rgba8())
    }
}
