use std::fs::{self, File};
use std::io::{self,Read};
 use std::ops::Add;
use std::{env, path};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::fs::read_link;
use qrcode::QrCode;
use image::Luma;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);
fn main() -> io::Result<()>{
   
fs::remove_dir_all("./images_out")?;
fs::create_dir_all("./images_out")?;
    // https://stackoverflow.com/questions/62840979/how-do-i-get-the-filename-of-an-open-stdfsfile-in-rust

    let paths = fs::read_dir("/home/goseberry/binarycoder/entry")?;
    let mut file_name = String::new();
    println!("{:?}",paths);
    for entry in paths{
        // println!("path is {:?}",entry?.path());
        let path: PathBuf = entry?.path();
        file_name = path.into_os_string().into_string().unwrap();

    }

    println!("\n File name is {}",file_name);
    // to do make so it reads file name only 
    // println!("file {}",file_name.split_off(file_name.find("entry").unwrap() + "entry".to_owned().len() + 1));
    // println!("file {}",file_name.split_off(file_name.find("entry").unwrap() + "entry".to_owned().len() + 1));

     // test encode txts only for now;
    // let mut file_path = "test.asm.txt"; // to do check if it has only one .txt and if it has more then check number of dots in it and check if it has a .txt if there is only one txt and one dot then it is a txt else put another .txt to it 
    // let mut file = File::open(file_path)?;
    let file_path1: String =     file_name.split_off(file_name.find("entry").unwrap() + "entry".to_owned().len() + 1);
    println!("file path1 {}",file_path1);
    let mut file_path1_clone = file_path1.clone();
    let file_path2: String = "./entry/".to_owned();
    let file_path = file_path2 + &file_path1_clone;
    // let a = check_one(file_path1_clone);

    
    println!("file path is {}",file_path);
    let mut file_path = &file_path;
    match File::open(file_path){
        Ok(T) =>{



// reading all its contents into a buffer 





                let my_string = fs::read_to_string(file_path)?;

                // println!(" file contents are {}",my_string);


            let image_folder: String = creating_folder_images()?;

            if is_it_a_modified_txt(&file_path1_clone){

    // println!("new {}",file_path.replace(".txt", ""));
        //  println!("path is {}",file_path);
         let test_v = file_path.clone();

    remove_txt_name(&test_v)?;
            }else{

            
// reading all its contents into a buffer 

//  get the file format so .asm or .txt
            // println!("The file format is:{}",); 
                let file_format = file_path1_clone.split_off(file_path1_clone.find(".").unwrap());
                println!("\n \n \n File format and it is modified {}",file_path1);


                let my_string = fs::read_to_string(file_path)?;
                let mut my_vec: Vec<String> = Vec::new();
                my_vec.push(file_path1);
                my_vec.push(my_string);
                println!(" file contents are {:?}",my_vec);
                let mut mystring = String::new();
                // print!("{:#?}",mystring.join(my_vec.iter()));
                mystring += &my_vec[0]; 
                mystring += " | ";
                mystring += &my_vec[1];
                println!("\n \n mystring \n {}",mystring);
                let create_folder: String = creating_folder()?;
                let output_file_path = creating_a_file(&create_folder, &my_vec[0], mystring)?; // OUTPUT folders created encoded files path
                println!("Output file path is \n ::{:#?}",output_file_path.clone());
                //Filename first And Output contents next
                let read_and_delete_response = reading_output_file_and_deleting_old(output_file_path.clone())?; 
                std::fs::write(&read_and_delete_response.0, read_and_delete_response.1)?;

                making_qr_codes(read_and_delete_response.0.into())?;
















    // creating the .txt extension to it 
// make a functionn out of this 
    println!("new {}",file_path.to_owned() + ".txt");
    let new_file_name = file_path.to_owned() + ".txt";
    fs::rename(file_path, new_file_name)?;


    























            }




    
        }
        Err(e) =>{
    println!("new {}",file_path.replace(".txt", ""));
    let new_file_name = file_path.replace(".txt", "");
    fs::rename(file_path, new_file_name)?;

        }
    }


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

fn creating_folder() -> io::Result<(String)>{
        let a = std::env::current_dir()?;

    let patth = a.to_str().unwrap().to_owned() + "/Output";
    let b: () = std::fs::create_dir_all(patth.clone())?;
    // println!("current dir is {:#?}",a);
    // println!("path is is {:#?}",patth);
    Ok(patth)
}

fn creating_folder_images() -> io::Result<(String)>{
        let a = std::env::current_dir()?;

    let patth = a.to_str().unwrap().to_owned() + "/images_out";
    let b: () = std::fs::create_dir_all(patth.clone())?;
    // println!("current dir is {:#?}",a);
    // println!("path is is {:#?}",patth);
    Ok(patth)
}

fn creating_a_file(path:&str,file_name:&str,contents:String)-> io::Result<PathBuf>{
    let mut file_namee: String = file_name.to_owned();
    file_namee += ".txt";
    let file_path: PathBuf = PathBuf::from(path).join(&file_namee);
    println!("D:creatigng_a_file path is {:#?}",file_path);
    
    std::fs::write(&file_path, contents)?; // change asd to the contents of the file
    // writing the contents of it into a file
    println!("the path we are crating is {:#?}",file_path);
    Ok(file_path)
    
    
}

fn remove_txt_name(path:&str) -> io::Result<()>{

     println!("new {}",path.replace(".txt", ""));
     println!("path is {}",path);
    let new_file_name = path.replace(".txt", "");
    fs::rename(path, new_file_name)?;
    Ok(())
}
// MAKE A FILE INSIDE THE FOLDER IT CREATED using the vec the create folder supplies the output dirr path that it has created 
// make the original.asm.txt

fn reading_output_file_and_deleting_old(path:PathBuf) -> io::Result<(String,String)>{

            let original_output_path_clone = path.clone();
                    // we can get the file_name from the output file path and also the file contents we can reconstruct the original file now 
                let mut output_contents: String = fs::read_to_string(path)?; 
                println!("Output file contents \n {:#?} ",output_contents.split_at(output_contents.find("|").unwrap() + 2)  );
                // getting file name out of the file
                let mut output_splited: (&str, &str) = output_contents.split_at(output_contents.find("|").unwrap() + 2);
                let mut file_name: &str = output_splited.0;
                let file_name: String = file_name.replace("|", "").trim_end().to_string();
                println!("file name is {}",file_name);

                // modifying file contents so it won't have the first part of the txt later adding it into a function
                let output_contents = output_splited.1;
                fs::remove_file(original_output_path_clone)?;
                fs::remove_dir_all("./Output")?;
                fs::create_dir_all("./Output")?;
                
        //Filename first And Output contents next
    Ok(("./Output/".to_owned() + &file_name,output_contents.to_string()))
}

fn making_qr_codes(path:PathBuf) -> io::Result<()>{
    
    // Devide the len with 128 and the rest which is not even round it up and pack that many bytes 
    let mut contents = String::new();
    contents = fs::read_to_string(path)?;

    // println!("CONTENTS OF THE FILE{}",contents);
    // println!("the len of it is {}:",contents.len());

    let mut contents_cloned = contents.clone();
    let mut bytes_added:usize = 0;
    let mut holder : Vec<String> = Vec::new();
    for i in 1..contents.len() / 128 + 2{
        // println!("Holder is {:#?}",holder);
        if bytes_added + 128 > contents.len(){
            let remaning_bytes = contents.len() - bytes_added;
            // println!("We got filled up remaning bytes are {}: ",remaning_bytes);
            // println!("19 remaning {},",&contents.to_string()[bytes_added..(remaning_bytes + bytes_added)]);
            let a: &str = &contents.to_string()[bytes_added..(remaning_bytes + bytes_added)];
            let a: String = a.to_string();
            // println!("a is {}?",a);
            holder.clear();
                holder.push(a);
                create_qr_code(holder.clone())?;

        // break; // MAKE QR CODE OUT OF THIS 


        }else{
            bytes_added += 128;
            // println!("bytes added is {}",bytes_added);
            // bytes_added = bytes_added +  128;
            // println!("i*128 {}",i*128); 
            if bytes_added == 128{
                // println!("128 first char of the contents_cloned is {},",&contents.to_string()[0..(i*128)]);
                // holder.push((&contents.to_string()[0..(i*128)]).to_string());
                let a = &contents.to_string()[0..(i*128)];
                let a: String = a.to_string();
                holder.clear();
                holder.push(a);
                create_qr_code(holder.clone())?;
            }else{

                // holder.push((&contents.to_string()[bytes_added..(i*128)]).to_string());
                                // holder.clear();

                let a = &contents.to_string()[bytes_added -128 ..(i*128)];
                let a: String = a.to_string();
                // println!("asdasdasd aaaaaa is a:{}",a);
                holder.clear();
                holder.push(a);
                create_qr_code(holder.clone())?;
                // println!("128 first char of the contents_cloned is {},",&contents_cloned.to_string()[bytes_added- 128..(i*128)]);

            }
// MAKE QR CODE OUT OF THIS
            
        }
    }
    // get the all size and make it equals to a vrabiée remove 128 from it if we go minus we know we went over so calculate the difrence between the rest which is positiv and make qr codes out of them
    // todo!()


    Ok(())
}

pub fn create_qr_code(input:Vec<String>) -> io::Result<()>{
    let current = COUNTER.load(Ordering::Relaxed); // read
      COUNTER.fetch_add(1, Ordering::Relaxed); // increment
    println!("creating a qr code out of {:#?}",input.clone());
    let data = &input[0];
    // println!("data is {}",data);
    let code = QrCode::new(format!("{data} |&:{current}|")).unwrap();
let image = code.render::<Luma<u8>>().build();

image.save(format!("./images_out/{current}.png")).unwrap(); 
    Ok(())
}