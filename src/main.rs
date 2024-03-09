use aes::cipher::BlockDecryptMut;
use aes::cipher::KeyIvInit;
use clap::Parser;
use miette::IntoDiagnostic;
use miette::miette;

#[derive(Debug, clap::Parser)]
#[command(about, version)]
struct Arguments {
    #[arg(long, short)]
    device_id: String,

    #[arg(long, short)]
    user_idx: String,
}

#[derive(Debug)]
enum BookFormat {
    EPUB,
    PDF,
}

impl BookFormat {
    fn from<P: AsRef<std::path::Path>>(path: P) -> miette::Result<Self> {
        match path.as_ref().extension() {
            Some(extension) => {
                match extension.to_str().ok_or_else(|| miette!("invalid extension"))? {
                    "epub" => Ok(BookFormat::EPUB),
                    "pdf" => Ok(BookFormat::PDF),
                    _ => Err(miette!("not a book file: {}", path.as_ref().display())),
                }
            },
            None => Err(miette!("not a book file: {}", path.as_ref().display())),
        }
    }

    fn extension(&self) -> &str {
        match self {
            Self::EPUB => "epub",
            Self::PDF => "pdf",
        }
    }
}

enum FileKind {
    Book,
    Data,
}

#[derive(Debug)]
struct BookInfo {
    format: BookFormat,
    id: std::ffi::OsString,
    path: std::path::PathBuf,
}

impl BookInfo {
    fn id<P: AsRef<std::path::Path>>(path: P) -> miette::Result<std::ffi::OsString> {
        Ok(path.as_ref().file_name().ok_or_else(|| miette!("invalid id"))?.to_owned())
    }

    fn format<P: AsRef<std::path::Path>>(path: P) -> miette::Result<BookFormat> {
        std::fs::read_dir(&path).into_diagnostic()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .map(BookFormat::from)
            .filter_map(Result::ok)
            .take(1)
            .next()
            .ok_or_else(|| miette!("not a book path: {}", path.as_ref().display()))
    }

    fn from<P: AsRef<std::path::Path>>(path: P) -> miette::Result<Self> {
        Ok(Self {
            format: Self::format(&path)?,
            id: Self::id(&path)?,
            path: path.as_ref().to_owned(),
        })
    }

    fn file_path(&self, kind: &FileKind) -> std::path::PathBuf {
        let mut path = std::path::PathBuf::from(&self.path).join(&self.id);
        path.set_file_name(&self.id);
        match kind {
            FileKind::Book => path.set_extension(self.format.extension()),
            FileKind::Data => path.set_extension("dat"),
        };
        path
    }

    fn file_name(&self, kind: &FileKind) -> std::ffi::OsString {
        self.file_path(&kind).file_name().unwrap().to_owned()
    }
}

fn verify(arguments: &Arguments) -> miette::Result<()> {
    if arguments.device_id.len() != 36 {
        return Err(miette!("invalid device id: {}", arguments.device_id));
    }
    if arguments.user_idx.is_empty() {
        return Err(miette!("invalid user idx"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn library_path(user_idx: &str) -> miette::Result<std::path::PathBuf> {
    Ok(std::path::PathBuf::from(std::env::var("HOME").into_diagnostic()?)
        .join("Library")
        .join("Application Support")
        .join("Ridibooks")
        .join("library")
        .join(format!("_{user_idx}")))
}

#[cfg(target_os = "windows")]
fn library_path(user_idx: &str) -> miette::Result<std::path::PathBuf> {
    Ok(std::path::PathBuf::from(std::env::var("APPDATA").into_diagnostic()?)
        .join("Ridibooks")
        .join("library")
        .join(format!("_{user_idx}")))
}

#[cfg(not(target_os = "macos"))]
#[cfg(not(target_os = "windows"))]
fn library_path(user_idx: &str) -> miette::Result<std::path::PathBuf> {
    unimplemented!("library_path()");
}

fn book_infos<P: AsRef<std::path::Path>>(path: P) -> miette::Result<Vec<BookInfo>> {
    Ok(std::fs::read_dir(&path).into_diagnostic()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(BookInfo::from)
        .filter_map(Result::ok)
        .collect())
}

fn decrypt_key(book_info: &BookInfo, device_id: &str) -> miette::Result<[u8; 16]> {
    let data_file = std::fs::read(book_info.file_path(&FileKind::Data)).into_diagnostic()?;

    let mut key = [0; 16];
    key.copy_from_slice(&device_id.as_bytes()[0..16]);

    let mut iv = [0; 16];
    iv.copy_from_slice(&data_file[0..16]);

    let plaintext = cbc::Decryptor::<aes::Aes128>::new(&key.into(), &iv.into())
        .decrypt_padded_vec_mut::<aes::cipher::block_padding::Pkcs7>(&data_file[16..])
        .map_err(|error| miette!("{error}"))?;

    let mut result = [0; 16];
    result.copy_from_slice(&std::str::from_utf8(&plaintext).into_diagnostic()?[68..84].as_bytes());

    Ok(result)
}

fn decrypt_book(book_info: &BookInfo, key: &[u8; 16]) -> miette::Result<Vec<u8>> {
    let book_file = std::fs::read(book_info.file_path(&FileKind::Book)).into_diagnostic()?;

    let mut iv = [0; 16];
    iv.copy_from_slice(&book_file[0..16]);

    cbc::Decryptor::<aes::Aes128>::new(key.into(), &iv.into())
        .decrypt_padded_vec_mut::<aes::cipher::block_padding::Pkcs7>(&book_file[16..])
        .map_err(|error| miette!("{error}"))
}

fn decrypt(book_info: &BookInfo, device_id: &str) -> miette::Result<()> {
    let file_name = book_info.file_name(&FileKind::Book);
    let book_contents = decrypt_book(&book_info, &decrypt_key(&book_info, &device_id)?)?;
    std::fs::write(&file_name, &book_contents).into_diagnostic()
}

fn decrypt_with_progress(book_info: &BookInfo, device_id: &str) -> miette::Result<()> {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.enable_steady_tick(core::time::Duration::from_millis(100));
    spinner.set_style(
        indicatif::ProgressStyle::with_template("{spinner} {msg}")
            .into_diagnostic()?
            .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷", "⣿"])
    );

    let file_name = book_info.file_name(&FileKind::Book);
    spinner.set_message(format!("Decrypting {:?}", &file_name));

    let result = decrypt(&book_info, &device_id);
    let result_status = if result.is_ok() { "✔︎" } else { "✘" };
    spinner.finish_with_message(format!("Decrypting {:?} {}", &file_name, &result_status));

    result
}

fn main() -> miette::Result<()> {
    let arguments = Arguments::parse();
    verify(&arguments)?;
    let _ = book_infos(&library_path(&arguments.user_idx)?)?
        .iter()
        .map(|book_info| decrypt_with_progress(&book_info, &arguments.device_id))
        .collect::<Vec<_>>();
    Ok(())
}
