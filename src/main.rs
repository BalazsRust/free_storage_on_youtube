use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;

use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use image::{imageops, DynamicImage, Rgb, RgbImage};
use qrism::reader::detect_hc_qr;
use qrism::QRBuilder;
use rand::RngCore;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, serde::Deserialize)]
pub struct Config {
    #[serde(default = "d_byte_len")] pub byte_len: usize,
    #[serde(default = "d_width")]    pub width: u32,
    #[serde(default = "d_height")]   pub height: u32,
    #[serde(default = "d_fps")]      pub fps: u32,
    #[serde(default = "d_frames_per_image")] pub frames_per_image: u32,
    #[serde(default)]                pub encrypt: bool,
    #[serde(default)]                pub password: String,
    #[serde(default)]                pub error_correction: i8,
    #[serde(default)]                pub crf: u8,

    // side length of one QR tile in pixels. None = as big as the frame allows.
    #[serde(default = "d_scale_to")] pub scale_to: Option<u32>,
}

fn d_byte_len() -> usize { 5000 }
fn d_width() -> u32 { 1920 }
fn d_height() -> u32 { 1080 }
fn d_fps() -> u32 { 240 }
fn d_frames_per_image() -> u32 { 1 }
fn d_scale_to() -> Option<u32> { Some(1080) }
impl Default for Config {
    fn default() -> Self {
        Self {
            byte_len: 5000, width: 1920, height: 1080, fps: 240,
            frames_per_image: 1, encrypt: false,
            error_correction: 1,
            crf: 28,
            password: String::new(), scale_to: Some(1080),
        }
    }
}

const BYTE_LEN: usize = 5000;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const CRF: u8 = 28;

const FPS: u32 = 240;
const FRAMES_PER_IMAGE: u32 = 1; // each image is shown for 1 second
const SCALE_TO: Option<u32> = Some(1080); // QR tile size in px; None = as big as the frame allows
const ENCRYPT: bool = true;
const PASSWORD: &str = "123456789";

#[tokio::main]
async fn main() -> io::Result<()> {
    // wipe outputs, never wipe inputs
    fs::remove_dir_all("./images_out").unwrap_or(());
    fs::remove_dir_all("./FileOut").unwrap_or(());
    fs::create_dir_all("./entry").unwrap();
    fs::create_dir_all("./images_out").unwrap();
    fs::create_dir_all("./FileOut").unwrap();
    fs::create_dir_all("./Output").unwrap();

    // let mut input = String::new();
    // println!("To Encode Put a the file you want to encode into entry to get it check Output\n To Decode the video must be ./Output/output.mp4 and check FileOut\nEnter 1 : Encode(Making a video)\nEnter 2: Decoding(Getting Default Data Out)");
    // io::stdin().read_line(&mut input)?;
    // println!("You entered line {}", input);
    let input = 3;

    let cfg = Config {
        byte_len: BYTE_LEN,
        width: WIDTH,
        height: HEIGHT,
        fps: FPS,
        frames_per_image: FRAMES_PER_IMAGE,
        encrypt: ENCRYPT,
        password: PASSWORD.to_string(),
        error_correction: 1,
        crf: CRF,
        scale_to: SCALE_TO,
    };

    // decode branch
    if input == 2 {
        slice_video(&cfg, "./Output/output.mp4")?;
        reading_images_from_video(&cfg)?;
        return Ok(());
    } else if input == 3 {
        let state = Arc::new(AsyncMutex::new(()));
        let app = Router::new()
            .route("/encode", post(encode_handler))
            .route("/decode", post(decode_handler))
            .with_state(state)
            .layer(DefaultBodyLimit::max(1024 * 1024 * 1024 * 10)) // 10GB uploads
            .layer(tower_http::cors::CorsLayer::permissive());

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        println!("API running on http://0.0.0.0:3000");
        axum::serve(listener, app).await?;
        return Ok(());
    }

    // encode branch
    println!("You must put a file inside entry. Press Enter to continue");
    io::stdin().read_line(&mut String::new())?;

    let mut file_name = String::new();
    for entry in fs::read_dir("./entry")? {
        let p = entry?.path();
        file_name = p.file_name().unwrap().to_string_lossy().into_owned();
    }
    assert!(!file_name.is_empty(), "no file found in ./entry");
    println!("Encoding file: {}", file_name);

    let archive = "./entry.tar.gz";
    pack_entry(archive)?;

    let mut to_encode = PathBuf::from(archive);
    if ENCRYPT {
        let data = fs::read(archive)?;
        let encrypted = encrypt_bytes(PASSWORD, &data);
        let enc_path = "./entry.tar.gz.enc";
        fs::write(enc_path, encrypted)?;
        to_encode = PathBuf::from(enc_path);
    }
    let encode_name = to_encode.file_name().unwrap().to_string_lossy().into_owned();
    making_qr_codes(&cfg, &encode_name, to_encode)?;

    video_maker(&cfg)?;

    println!("Done check the --> ./Output/output.mp4");
    Ok(())
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .expect("key derivation failed");
    key
}

fn encrypt_bytes(password: &str, data: &[u8]) -> Vec<u8> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ct = cipher.encrypt(Nonce::from_slice(&nonce), data).expect("encrypt failed");
    let mut out = salt.to_vec();
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    out
}

fn decrypt_bytes(password: &str, data: &[u8]) -> Vec<u8> {
    let (salt, rest) = data.split_at(16);
    let (nonce, ct) = rest.split_at(12);
    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    cipher.decrypt(Nonce::from_slice(nonce), ct)
        .expect("wrong password or corrupted data")
}

// bytes <-> printable ASCII. qrism hands back a String, so the payload has to be text.
// To switch encoding, only these two functions change.
fn encode_text(bytes: &[u8]) -> String {
    String::from_utf8(base91::slice_encode(bytes)).expect("base91 output is ASCII")
}

fn decode_text(text: &str) -> Vec<u8> {
    base91::slice_decode(text.as_bytes())
}

fn reading_images_from_video(cfg: &Config) -> io::Result<()> {
    reading_qr_codes(cfg)?;
    Ok(())
}

fn qr_byte_len(cfg: &Config) -> usize {
    if cfg.byte_len > 6500 { 6496 } else { cfg.byte_len }
}

fn making_qr_codes(cfg: &Config, file_name_and_format: &str, path: PathBuf) -> io::Result<()> {
    let byte_len = qr_byte_len(cfg);
    let bytes = fs::read(&path)?;
    let encoded = encode_text(&bytes);

    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(byte_len + 4)
        .map(|c| std::str::from_utf8(c).expect("base91 output is ASCII"))
        .collect();

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    std::thread::scope(|s| -> io::Result<()> {
        let handles: Vec<_> = (0..num_threads)
            .map(|w| {
                let chunks = &chunks;
                s.spawn(move || -> io::Result<()> {
                    let mut i = w;
                    while i < chunks.len() {
                        create_qr_code(cfg, i, file_name_and_format, chunks[i])?;
                        i += num_threads;
                    }
                    Ok(())
                })
            })
            .collect();

        for h in handles {
            h.join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "qr worker panicked"))??;
        }
        Ok(())
    })
}

pub fn create_qr_code(cfg: &Config, current: usize, file_name_and_format: &str, data: &str) -> io::Result<()> {
    println!("creating a qr code out of chunk {}", current);

    let mut qr_data = format!("{data} |&:{current}|");
    if current == 0 {
        qr_data = format!("{data} |&:{current}|{file_name_and_format}"); // filename only in first chunk
    }

    let pad_to = qr_byte_len(cfg) + 4 + file_name_and_format.len() + 32;
    let qr_data = format!("{qr_data:<pad_to$}");

    let ecl = match cfg.error_correction {
        1 => qrism::ECLevel::M,
        2 => qrism::ECLevel::Q,
        3 => qrism::ECLevel::H,
        _ => qrism::ECLevel::L,
    };

    let qr = QRBuilder::new(qr_data.as_bytes())
        .high_capacity(true)
        .ec_level(ecl)
        .build()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "chunk {current}: {e} (payload {} B, error_correction {}). \
                     Lower the chunk size or use a lower error-correction level.",
                    qr_data.len(),
                    cfg.error_correction
                ),
            )
        })?;

    qr.to_image(8)
        .save(format!("./images_out/{current}.png"))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(())
}

// ---------- tiling: several QR codes per video frame ----------

// side length of one tile; never bigger than the frame
fn tile_size(cfg: &Config, w: u32, h: u32) -> u32 {
    let full = w.min(h);
    cfg.scale_to.unwrap_or(full).clamp(1, full)
}

fn grid(w: u32, h: u32, t: u32) -> (u32, u32) {
    ((w / t).max(1), (h / t).max(1))
}

// top-left corner of tile k, tiles are spread evenly over the frame
fn tile_pos(w: u32, h: u32, t: u32, k: u32) -> (u32, u32) {
    let (cols, rows) = grid(w, h, t);
    let gap_x = w.saturating_sub(cols * t) / (cols + 1);
    let gap_y = h.saturating_sub(rows * t) / (rows + 1);
    (gap_x + (k % cols) * (t + gap_x), gap_y + (k / cols) * (t + gap_y))
}

fn is_blank(img: &DynamicImage) -> bool {
    img.to_luma8().pixels().all(|p: &image::Luma<u8>| p.0[0] < 24)
}

fn read_tile(tile: &DynamicImage) -> Option<String> {
    let mut res = detect_hc_qr(tile);
    let symbol = res.symbols().first_mut()?;
    symbol.decode().ok().map(|(_, msg)| msg)
}

// every readable QR message found in one video frame
fn read_frame(cfg: &Config, path: &str) -> Vec<String> {
    let img = image::open(path).expect("Error with img open");
    let (w, h) = (img.width(), img.height()); // use the real frame size, not the config
    let t = tile_size(cfg, w, h);
    let (cols, rows) = grid(w, h, t);

    let mut out = Vec::new();
    for k in 0..cols * rows {
        let (x, y) = tile_pos(w, h, t, k);
        let tile = img.crop_imm(x, y, t, t);
        if is_blank(&tile) {
            continue; // unused tile in the last frame
        }
        if let Some(msg) = read_tile(&tile) {
            out.push(msg);
        }
    }
    out
}

// chunk number out of "...data |&:12|"
fn chunk_index(msg: &str) -> Option<usize> {
    let p = msg.rfind("|&:")? + 3;
    msg[p..].split('|').next()?.parse().ok()
}

pub fn reading_qr_codes(cfg: &Config) -> io::Result<()> {
    let mut files: Vec<String> = Vec::new();
    for entry in fs::read_dir("./images_out")? {
        if let Some(name) = entry?.file_name().to_str() {
            files.push(name.to_string());
        }
    }

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);

    // keyed by chunk number: frame order, tile order and repeated frames don't matter
    let mut by_idx: BTreeMap<usize, String> = BTreeMap::new();
    for batch in files.chunks(num_threads) {
        let found: Vec<Vec<String>> = std::thread::scope(|s| {
            let handles: Vec<_> = batch
                .iter()
                .map(|f| s.spawn(move || read_frame(cfg, &format!("./images_out/{f}"))))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("thread panicked"))
                .collect()
        });
        for msg in found.into_iter().flatten() {
            if let Some(idx) = chunk_index(&msg) {
                by_idx.entry(idx).or_insert(msg);
            }
        }
    }

    let last = *by_idx.keys().next_back().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "no QR codes could be read from the video")
    })?;
    let missing: Vec<usize> = (0..=last).filter(|i| !by_idx.contains_key(i)).collect();
    if !missing.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} chunk(s) could not be read, first missing: {:?}",
                missing.len(),
                &missing[..missing.len().min(10)]
            ),
        ));
    }

    reconsturcting_qr_to_file(cfg, by_idx.into_values().collect())
}

pub fn reconsturcting_qr_to_file(cfg: &Config, vector: Vec<String>) -> io::Result<()> {
    let file_name = get_name("|", &vector[0]);

    let mut one_string = String::new();
    for i in vector {
        one_string += &normal_string("|", i);
    }

    let bytes = decode_text(&one_string);
    let bytes = if file_name.ends_with(".enc") {
        decrypt_bytes(&cfg.password, &bytes)
    } else {
        bytes
    };

    fs::create_dir_all("./FileOut")?;
    let out_path = format!("./FileOut/{}", file_name.trim_end_matches(".enc"));
    fs::write(&out_path, bytes)?;
    if out_path.ends_with(".tar.gz") {
        unpack_archive(&out_path, "./FileOut")?;
        fs::remove_file(&out_path)?;
    }
    Ok(())
}

pub fn normal_string(_find: &str, input: String) -> String {
    let idx = input.rfind("|&:").unwrap_or(input.len());
    input[..idx.saturating_sub(1)].to_string()
}

pub fn get_name(_find: &str, input: &str) -> String {
    let idx = input.rfind('|').unwrap_or(0);
    input[idx + 1..].trim().to_string()
}

async fn encode_handler(
    State(state): State<Arc<AsyncMutex<()>>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let _guard = state.lock().await; // one encode at a time

    clean_workspace().ok();
    let mut cfg = Config::default();
    let mut saved_any = false;

    fs::remove_dir_all("./entry").unwrap_or(());
    fs::create_dir_all("./entry").unwrap();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            // JSON config field, e.g. {"byte_len":3000,"fps":120,"encrypt":true,"password":"hunter2"}
            Some("config") => {
                if let Ok(text) = field.text().await {
                    if let Ok(parsed) = serde_json::from_str::<Config>(&text) {
                        cfg = parsed;
                    }
                }
            }
            // every other field = a file upload
            _ => {
                let Some(name) = field.file_name().map(|s| s.to_string()) else { continue };
                let data = match field.bytes().await {
                    Ok(d) => d,
                    Err(e) => return (StatusCode::BAD_REQUEST, format!("upload failed: {e}")).into_response(),
                };
                if fs::write(format!("./entry/{name}"), &data).is_err() {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "could not save file").into_response();
                }
                saved_any = true;
            }
        }
    }

    if !saved_any {
        return (StatusCode::BAD_REQUEST, "no files uploaded").into_response();
    }

    let result = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let archive = "./entry.tar.gz";
        pack_entry(archive).map_err(|e| e.to_string())?;

        let mut to_encode = PathBuf::from(archive);
        if cfg.encrypt {
            let data = fs::read(archive).map_err(|e| e.to_string())?;
            let encrypted = encrypt_bytes(&cfg.password, &data);
            fs::write("./entry.tar.gz.enc", encrypted).map_err(|e| e.to_string())?;
            to_encode = PathBuf::from("./entry.tar.gz.enc");
        }
        let name = to_encode.file_name().unwrap().to_string_lossy().into_owned();
        making_qr_codes(&cfg, &name, to_encode).map_err(|e| e.to_string())?;
        video_maker(&cfg).map_err(|e| e.to_string())?;

        fs::read("./Output/output.mp4").map_err(|e| e.to_string())
    })
    .await;

    match result {
        Ok(Ok(bytes)) => (
            [(header::CONTENT_TYPE, "video/mp4")],
            bytes,
        ).into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("pipeline panicked: {e}")).into_response(),
    }
}

async fn decode_handler(
    State(state): State<Arc<AsyncMutex<()>>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let _guard = state.lock().await;
    clean_workspace().ok();

    let mut cfg = Config::default();
    let mut video_bytes: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("config") => {
                if let Ok(text) = field.text().await {
                    if let Ok(parsed) = serde_json::from_str::<Config>(&text) {
                        cfg = parsed;
                    }
                }
            }
            _ => {
                if let Ok(data) = field.bytes().await {
                    video_bytes = Some(data.to_vec());
                }
            }
        }
    }
    let Some(video) = video_bytes else {
        return (StatusCode::BAD_REQUEST, "no video uploaded").into_response();
    };

    let result = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        fs::create_dir_all("./Output").map_err(|e| e.to_string())?;
        fs::write("./Output/output.mp4", video).map_err(|e| e.to_string())?;
        slice_video(&cfg, "./Output/output.mp4").map_err(|e| e.to_string())?;
        reading_qr_codes(&cfg).map_err(|e| e.to_string())?;

        // pack everything recovered into one tar.gz to return
        let archive = "./FileOut.tar.gz";
        let file = File::create(archive).map_err(|e| e.to_string())?;
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(".", "./FileOut").map_err(|e| e.to_string())?;
        tar.into_inner().map_err(|e| e.to_string())?.finish().map_err(|e| e.to_string())?;
        let bytes = fs::read(archive).map_err(|e| e.to_string())?;
        fs::remove_file(archive).map_err(|e| e.to_string())?;
        Ok(bytes)
    })
    .await;

    match result {
        Ok(Ok(bytes)) => (
            [(header::CONTENT_TYPE, "application/gzip")],
            bytes,
        ).into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("pipeline panicked: {e}")).into_response(),
    }
}

fn video_maker(cfg: &Config) -> io::Result<()> {
    fs::remove_dir_all("./Output").unwrap();
    fs::create_dir_all("./Output").unwrap();

    let dir = "./images_out";

    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map_or(false, |e| e == "png"))
        .collect();
    paths.sort_by_key(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(u64::MAX)
    });

    if paths.is_empty() {
        eprintln!("No PNG files found in {dir}");
        return Ok(());
    }

    // as many QR images per video frame as fit in the resolution
    let t = tile_size(cfg, cfg.width, cfg.height);
    let (cols, rows) = grid(cfg.width, cfg.height, t);
    let groups: Vec<Vec<PathBuf>> = paths
        .chunks((cols * rows) as usize)
        .map(|g| g.to_vec())
        .collect();
    println!("Found {} images -> {} frames ({}x{} tiles of {}px)", paths.len(), groups.len(), cols, rows, t);
    let groups = Arc::new(groups);

    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "rawvideo",
            "-pix_fmt", "rgb24",
            "-s", &format!("{}x{}", cfg.width, cfg.height),
            "-r", &cfg.fps.to_string(),
            "-i", "-",
            "-c:v", "libx264",
            "-crf", &cfg.crf.to_string(),
            "-preset", "veryfast",
            "-pix_fmt", "yuv420p",
            "-movflags", "+faststart",
            "./Output/output.mp4",
        ])
        .stdin(Stdio::piped())
        .spawn()?;

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(3);
    // keep queued raw frames to roughly 256 MB (a 4K frame is ~25 MB)
    let frame_bytes = cfg.width as usize * cfg.height as usize * 3;
    let queue = ((256usize << 20) / frame_bytes).clamp(2, num_threads * 2);
    let (tx, rx) = sync_channel::<(usize, Vec<u8>)>(queue);

    std::thread::scope(|s| -> io::Result<()> {
        // workers: decode PNGs -> one raw frame, send with its index
        for w in 0..num_threads {
            let tx = tx.clone();
            let groups = Arc::clone(&groups);
            s.spawn(move || {
                let mut i = w;
                while i < groups.len() {
                    let imgs: Vec<RgbImage> = groups[i]
                        .iter()
                        .filter_map(|p| match image::open(p) {
                            Ok(img) => Some(img.to_rgb8()),
                            Err(e) => {
                                eprintln!("Skipping {}: {e}", p.display());
                                None
                            }
                        })
                        .collect();
                    let raw = if imgs.is_empty() {
                        Vec::new()
                    } else {
                        to_canvas(cfg, &imgs).into_raw()
                    };
                    if tx.send((i, raw)).is_err() { break; }
                    i += num_threads;
                }
            });
        }
        drop(tx); // close channel when all workers finish

        // writer: single thread streams raw frames to ffmpeg IN ORDER
        let mut stdin = child.stdin.take().expect("Failed to open ffmpeg stdin");
        let mut buf: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut next = 0usize;

        while let Ok((i, raw)) = rx.recv() {
            buf.insert(i, raw);
            while let Some(raw) = buf.remove(&next) {
                if !raw.is_empty() { // failed frame was skipped
                    for _ in 0..cfg.frames_per_image.max(1) {
                        stdin.write_all(&raw)?;
                    }
                }
                next += 1;
            }
        }
        drop(stdin);
        Ok(())
    })?;

    let status = child.wait()?;
    println!("FFmpeg finished with: {status}");
    Ok(())
}

// paste the QR images onto one black frame, tile k goes to tile_pos(k)
fn to_canvas(cfg: &Config, imgs: &[RgbImage]) -> RgbImage {
    let t = tile_size(cfg, cfg.width, cfg.height);
    let mut canvas = RgbImage::from_pixel(cfg.width, cfg.height, Rgb([0, 0, 0]));
    for (k, img) in imgs.iter().enumerate() {
        let tile = imageops::resize(img, t, t, imageops::FilterType::Nearest);
        let (x, y) = tile_pos(cfg.width, cfg.height, t, k as u32);
        imageops::overlay(&mut canvas, &tile, x as i64, y as i64);
    }
    canvas
}

fn slice_video(_cfg: &Config, video_path: &str) -> io::Result<()> {
    fs::remove_dir_all("./images_out")?;
    fs::create_dir_all("./images_out")?;

    let status = Command::new("ffmpeg")
        .args([
            "-threads", "0",
            "-i", video_path,
            "-fps_mode", "passthrough",
            "-compression_level", "1",
            "./images_out/%d.png",
        ])
        .status()?;

    if !status.success() {
        eprintln!("ffmpeg failed with status: {}", status);
    }

    Ok(())
}

fn pack_entry(archive_path: &str) -> io::Result<()> {
    let file = File::create(archive_path)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(".", "./entry")?;
    tar.into_inner()?.finish()?;
    Ok(())
}

fn unpack_archive(archive_path: &str, out_dir: &str) -> io::Result<()> {
    let file = File::open(archive_path)?;
    let dec = flate2::read::GzDecoder::new(file);
    tar::Archive::new(dec).unpack(out_dir)
}

fn clean_workspace() -> io::Result<()> {
    for dir in ["./images_out", "./FileOut", "./Output", "./entry"] {
        fs::remove_dir_all(dir).ok();
        fs::create_dir_all(dir)?;
    }
    // stray artifacts from last run
    for f in ["./entry.tar.gz", "./entry.tar.gz.enc", "./FileOut.tar.gz"] {
        fs::remove_file(f).ok();
    }
    Ok(())
}