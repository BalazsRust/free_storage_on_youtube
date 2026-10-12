use std::fs::{self, File};
use std::io::{self, Read, Write};
 use std::ops::Add;
use std::process::exit;
use std::{env, path, vec};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::fs::read_link;
use std::sync::atomic::{AtomicU32, Ordering};
use base64::engine::general_purpose::STANDARD_PAD_INDIFFERENT;
use image::Delay;
use itertools::Itertools;
use qrism::{MaskPattern, QRBuilder};
use qrism::reader::detect_hc_qr;
use std::sync::Mutex;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::process::{Command, Stdio};
use image::{imageops, Rgb, RgbImage};
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::collections::HashMap;
use std::thread;

pub static MY_STRING: Mutex<String> = Mutex::new(String::new());   
static COUNTER: AtomicU32 = AtomicU32::new(0);
const BYTE_LEN: usize = 3004;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const FPS: u32 = 240;
const FRAMES_PER_IMAGE: u32 = 1 ; // each image is shown for 1 second
const SCALE_TO: Option<u32> = Some(1080); // upscale to a 1080x1080 square; None = original size


fn main() -> io::Result<()> {
    // wipe outputs, never wipe inputs
    fs::remove_dir_all("./images_out").unwrap_or(());
    fs::remove_dir_all("./FileOut").unwrap_or(());
    fs::create_dir_all("./entry").unwrap();
    fs::create_dir_all("./images_out").unwrap();
    fs::create_dir_all("./FileOut").unwrap();
    fs::create_dir_all("./Output").unwrap();


    let mut input = String::new();
    println!("To Encode Put a the file you want to encode into entry to get it check Output\n To Decode the video must be ./Output/output.mp4 and check FileOut\nEnter 1 : Encode(Making a video)\nEnter 2: Decoding(Getting Default Data Out)");
    io::stdin().read_line(&mut input)?;
    println!("You entered line {}", input);
    let input = input.trim().parse::<i32>().expect("You Entered Something Wrong");


    // decode branch
    if input == 2 {
        slice_video("./Output/output.mp4")?;
        reading_images_from_video()?;
        return Ok(());
    }


    // encode branch
    println!("You must put a file inside entry. Press Enter to continue");
    io::stdin().read_line(&mut String::new())?;


    let mut file_path = PathBuf::new();
    let mut file_name = String::new();
    for entry in fs::read_dir("./entry")? {
        let p = entry?.path();
        file_name = p.file_name().unwrap().to_string_lossy().into_owned();
        file_path = p;
    }
    assert!(!file_name.is_empty(), "no file found in ./entry");
    println!("Encoding file: {}", file_name);



    making_qr_codes(&file_name, file_path)?;
    video_maker()?;


    println!("Done check the --> ./Output/output.mp4");
    Ok(())
}

fn reading_images_from_video() -> io::Result<()>{

    reading_qr_codes();

    // println!("Sex");
    Ok(())
}

fn is_it_a_modified_txt(a:&str)-> bool{
    let find_dots = a.split("");
    let mut dots:u64 = 1;
    for i in find_dots{
        if i == "."{
            dots += 1;
        }
    }
    println!("the number of dots are {}",dots);
    if dots > 2{
        println!("IT IS A MODIFIED TXT");
        true
    }else {
        println!("it isn't a txt");
        false
    }
    // true
}


fn making_qr_codes(file_name_and_format:&str,path:PathBuf) -> io::Result<()>{
    
    // let contents = fs::read_to_string(path)?;
      let bytes = fs::read(&path)?;
    let contents = STANDARD_PAD_INDIFFERENT.encode(bytes);
    let chars: Vec<char> = contents.chars().collect();
    let chunks: Vec<String> = chars.chunks(BYTE_LEN)
        .map(|c| c.iter().collect())
        .collect();

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    std::thread::scope(|s| {
        for w in 0..num_threads {
            let chunks = &chunks;
            s.spawn(move || {
                let mut i = w;
                while i < chunks.len() {
                    create_qr_code(i, file_name_and_format, &chunks[i]).unwrap();
                    i += num_threads;
                }
            });
        }
    });

    Ok(())
}
pub fn create_qr_code(current:usize,file_name_and_format:&str,data:&str) -> io::Result<()>{
    println!("creating a qr code out of chunk {}",current);

    let mut qr_data = format!("{data} |&:{current}|");
    if current == 0{
        qr_data = format!("{data} |&:{current}|{file_name_and_format}"); // filename only in first chunk
    }

    let pad_to = BYTE_LEN + file_name_and_format.len() + 32;
    qr_data = format!("{qr_data:<pad_to$}");

    let qr = QRBuilder::new(qr_data.as_bytes())
    .high_capacity(true)
    .ec_level(qrism::ECLevel::M)
    .build()
    .expect("Error while generaitng qr code");

    let img = qr.to_image(8);
    img.save(format!("./images_out/{current}.png")).expect("Error while saving the image");
    Ok(())
}
pub fn reading_qr_codes()-> io::Result<()>{
    
    let path = "./images_out";
    let a = fs::read_dir(path)?;
    let mut n: Vec<String> = Vec::new();
    for i in a{
        let aaa: std::ffi::OsString = i?.file_name();
        let aaa = aaa.to_str().expect("Err");
        n.push(aaa.to_string());
    }
    remove_duplicates(&mut n);
    n.sort_by_key(|s| s.trim_end_matches(".png").parse::<u32>().unwrap_or(0));

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);

    let mut new_v: Vec<String> = Vec::new();
    for batch in n.chunks(num_threads) {
        let mut handles = vec![];
        for i in batch {
            let i = i.clone();
            handles.push(std::thread::spawn(move || {
                let path = format!("./images_out/{i}");
                println!("i in n is {}:", i);
                let img = image::open(path).expect("Error with img open");
                let mut res = detect_hc_qr(&img);
                if let Some(symbol) = res.symbols().first_mut() {
                    symbol.decode().expect("failed to read hq qr code").1
                } else {
                    String::new()
                }
            }));
        }
        for h in handles {
            let message = h.join().expect("thread panicked");
            if !message.is_empty() && new_v.last() != Some(&message) {
                new_v.push(message);
            }
        }
    }

    reconsturcting_qr_to_file(new_v);
    Ok(())
}

pub fn remove_duplicates(vector:&mut Vec<String>){
    // Takes a sorted vector as an input
    vector.dedup();
    // vector = vector.into_iter().unique();

}

pub fn reconsturcting_qr_to_file(vector:Vec<String>){
    let mut one_string = String::new();

    // println!("the vec is {:#?}:",vector);
    
    let file_name = get_name("|", &vector[0]);
    for i in vector{
        let i_len: usize = i.len();
        // println!("The string normalized {}",normal_string("|", i)); // Fix becaus elen can be more then this 
        one_string += &normal_string("|", i);
        // normal_string("|",i);
    }

    // println!("The file name is kkk {}",file_name);
    // exit(0);
    
    // println!("the one string is {}:",one_string);
    // it runs multiple times ebcause it is linked to create qr code for now i will change it later on
    fs::create_dir_all("./FileOut").unwrap();
    // Delay::from_numer_denom_ms(10, 1);
    // std::fs::write(format!("./FileOut/{file_name}"),one_string).unwrap(); // need to insert the name of it here
      let bytes = STANDARD_PAD_INDIFFERENT.decode(one_string.trim_end()).expect("bad base64");
    std::fs::write(format!("./FileOut/{file_name}"), bytes).unwrap();
}

pub fn normal_string(_find:&str,input:String) -> String{
    let idx = input.rfind("|&:").unwrap_or(input.len());
    input[..idx.saturating_sub(1)].to_string()
}

pub fn get_name(_find:&str,input:&str) -> String{
    let idx = input.rfind('|').unwrap_or(0);
    input[idx + 1..].trim().to_string()
}
















fn video_maker() -> io::Result<()>{


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
    println!("Found {} images", paths.len());
    let paths = Arc::new(paths);


    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "rawvideo",
            "-pix_fmt", "rgb24",
            "-s", &format!("{WIDTH}x{HEIGHT}"),
            "-r", &FPS.to_string(),
            "-i", "-",
            "-c:v", "libsvtav1",
            "-crf", "23",
            "-preset", "8",
            "-pix_fmt", "yuv420p",
            "-movflags", "+faststart",
            "./Output/output.mp4",
        ])
        .stdin(Stdio::piped())
        .spawn()?;


    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(3);
    let (tx, rx) = sync_channel::<(usize, Vec<u8>)>(num_threads * 2);


    std::thread::scope(|s| -> io::Result<()> {
        // workers: decode PNG -> raw frame, send with its index
        for w in 0..num_threads {
            let tx = tx.clone();
            let paths = Arc::clone(&paths);
            s.spawn(move || {
                let mut i = w;
                while i < paths.len() {
                    let raw = match image::open(&paths[i]) {
                        Ok(img) => to_canvas(&img.to_rgb8()).into_raw(),
                        Err(e) => {
                            eprintln!("Skipping {}: {e}", paths[i].display());
                            Vec::new()
                        }
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
                if !raw.is_empty() { // failed image was skipped
                    for _ in 0..FRAMES_PER_IMAGE {
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
fn to_canvas(img: &RgbImage) -> RgbImage {
    let img = match SCALE_TO {
        Some(s) => imageops::resize(img, s, s, imageops::FilterType::Nearest),
        None => img.clone(),
    };
    let mut canvas = RgbImage::from_pixel(WIDTH, HEIGHT, Rgb([0, 0, 0]));
    let x = (WIDTH as i64 - img.width() as i64) / 2;
    let y = (HEIGHT as i64 - img.height() as i64) / 2;
    imageops::overlay(&mut canvas, &img, x, y);
    canvas
}


fn slice_video(video_path: &str) -> io::Result<()> {
    fs::remove_dir_all("./images_out")?;
    fs::create_dir_all("./images_out")?;

    let status = Command::new("ffmpeg")
        .args([
            "-threads", "0",
            "-i", video_path,
            "-filter_threads", "0",   
            "-vf", &format!("fps={FPS}"),
            "-compression_level", "1",
            "./images_out/%d.png",
        ])
        .status()?;


    if !status.success() {
        eprintln!("ffmpeg failed with status: {}", status);
    }


    Ok(())
}