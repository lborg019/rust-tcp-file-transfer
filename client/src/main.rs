#[macro_use] extern crate lazy_static;

extern crate regex;
extern crate encoding;
extern crate console;
extern crate pbr;

use encoding::{Encoding, EncoderTrap};
use encoding::all::ASCII;
use std::error; //{env, error}
use std::net::TcpStream;
use std::io;
use std::str;
use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;
use std::fs;
use std::time::Duration;
use std::thread;
use pbr::ProgressBar;
use regex::Regex;

use std::fs::{DirEntry};
use std::path::Path;

use console::{Term, style};

struct RemoteFileList {
    f_name: String,
    f_size: String
}

fn ls_remote(command: &str, mut stream: &mut TcpStream) -> Result<String, Box<error::Error + Send + Sync>> {
        //get string size (in bytes)
    let mut string_size = command.len();
    string_size = string_size + 1;

    //println!("sending {} bytes", string_size);

    let string_size_str = string_size.to_string();

    //encode buffer to send size
    let mut string_size_bytes = try!(ASCII
                                         .encode(&string_size_str, EncoderTrap::Strict)
                                         .map_err(|x| x.into_owned()));
    string_size_bytes.push('\r' as u8);

    //prepare buffer to send message itself
    let mut command_bytes = try!(ASCII.encode(command, EncoderTrap::Strict)
                                      .map_err(|x| x.into_owned()));

    command_bytes.push('\r' as u8); //ending escape sequence

    //send message size:
    stream.write_all(&string_size_bytes).unwrap();

    //receive message size ACK:
    let mut ack_buf = [0u8; 8];
    stream.read(&mut ack_buf).unwrap();
    let ack_slice: &str = str::from_utf8(&mut ack_buf).unwrap(); //string slice
    let mut ack_str = ack_slice.to_string(); //convert slice to string
    let index: usize = ack_str.rfind('\r').unwrap();
    //println!("{:?} server ACK:", ack_str.split_off(index));
    format!("{:?}", ack_str.split_off(index)); 
    if ack_str != "ACK"{
        //println!("received ACK from server");
        // end with error, maybe set a timeout
    }

    //send message content
    stream.write_all(&command_bytes).unwrap();

    //receive message length:
    let mut buf = [0u8; 8]; //make it bigger if necessary
    stream.read(&mut buf).unwrap();

    //interpret the buffer contents into a string slice
    //let mut cl = buf.clone();
    let msg_len_slice: &str = str::from_utf8(&mut buf).unwrap(); //string slice
    let mut msg_len_str = msg_len_slice.to_string(); //convert slice to string

    /*
    CLEAN STRING:
    server might send message size smaller than buffer,
    which is usually the case when the server is sending
    the message size:

            buffer:     _ _ _ _ _ _ _ (bytes)
            message:    1 2 _ _ _ _ _ (bytes)

    (empty characters trail the meaningful characters)
    if this is the case, we shrink the string using .truncate()
    */

    let mut numeric_chars = 0;
    for c in msg_len_str.chars() {
        if c.is_numeric() == true {
            numeric_chars = numeric_chars + 1;
        }
    }

    //shrink:
    msg_len_str.truncate(numeric_chars);
    //println!("receiving {} bytes", msg_len_str);

    //send ACK:
    let mut ack_bytes = try!(ASCII.encode(&"ACK".to_string(), EncoderTrap::Strict).map_err(|x| x.into_owned()));
    ack_bytes.push('\r' as u8); //ending escape sequence
    stream.write_all(&ack_bytes).unwrap();

    //receive the file list:
    let mut remaining_data = msg_len_str.parse::<i32>().unwrap();
    let mut accumulator: String = String::new();
    let mut r = [0u8; 8]; //8 byte buffer

    //small message; receive as string
    while remaining_data != 0 {
        if remaining_data >= 8
        //slab >= 8 byte buffer
        {
            let slab = stream.read(&mut r);
            match slab {
                Ok(n) => {
                    let r_slice = str::from_utf8(&mut r).unwrap(); //string slice
                    accumulator.push_str(r_slice);
                    //println!("wrote {} bytes", n);
                    remaining_data = remaining_data - n as i32;
                }
                _ => {}
            }
        }
        /*
        option 1) receive and read a smaller buffer
        option 2) receive and read same buffer; truncate it to the smaller slab size

        since we cannot instantiate an array with a non-constant:
            e.g.: let mut r = [0u8; remainingData];
        it is better to just put the byte in the 8 byte buffer, and shrink it with
        .truncate() method before pushing to the String
        */
        else
        //slab < 8 byte buffer
        {
            let slab = stream.read(&mut r);
            match slab {
                Ok(n) => {
                    let s_slice = str::from_utf8(&mut r).unwrap(); //string slice
                    let mut s_str = s_slice.to_string(); //convert slice to string
                    s_str.truncate(n);
                    accumulator.push_str(&s_str);
                    //println!("wrote {} bytes", n);
                    remaining_data = remaining_data - n as i32;
                }
                _ => {}
            }
        }
    }
    let response = accumulator;
    Ok(response)
}

fn check_cmd(command: &str, mut stream: &mut TcpStream) -> Result<String, Box<error::Error + Send + Sync>> {

    //get string size (in bytes)
    let mut string_size = command.len();
    string_size = string_size + 1;

    println!("sending {} bytes", string_size);

    let string_size_str = string_size.to_string();

    //encode buffer to send size
    let mut string_size_bytes = try!(ASCII
                                         .encode(&string_size_str, EncoderTrap::Strict)
                                         .map_err(|x| x.into_owned()));
    string_size_bytes.push('\r' as u8);

    //prepare buffer to send message itself
    let mut command_bytes = try!(ASCII.encode(command, EncoderTrap::Strict)
                                      .map_err(|x| x.into_owned()));

    command_bytes.push('\r' as u8); //ending escape sequence

    //send message size:
    stream.write_all(&string_size_bytes).unwrap();

    //receive message size ACK:
    let mut ack_buf = [0u8; 8];
    stream.read(&mut ack_buf).unwrap();
    let ack_slice: &str = str::from_utf8(&mut ack_buf).unwrap(); //string slice
    let mut ack_str = ack_slice.to_string(); //convert slice to string
    let index: usize = ack_str.rfind('\r').unwrap();
    //println!("{:?} server ACK:", ack_str.split_off(index));
    format!("{:?}", ack_str.split_off(index)); 
    if ack_str == "ACK"{
        println!("received ACK from server");
    }

    //send message content
    stream.write_all(&command_bytes).unwrap();

    //receive message length:
    let mut buf = [0u8; 8]; //make it bigger if necessary
    stream.read(&mut buf).unwrap();

    //interpret the buffer contents into a string slice
    //let mut cl = buf.clone();
    let msg_len_slice: &str = str::from_utf8(&mut buf).unwrap(); //string slice
    let mut msg_len_str = msg_len_slice.to_string(); //convert slice to string

    /*
    CLEAN STRING:
    server might send message size smaller than buffer,
    which is usually the case when the server is sending
    the message size:

            buffer:     _ _ _ _ _ _ _ (bytes)
            message:    1 2 _ _ _ _ _ (bytes)

    (empty characters trail the meaningful characters)
    if this is the case, we shrink the string using .truncate()
    */

    let mut numeric_chars = 0;
    for c in msg_len_str.chars() {
        if c.is_numeric() == true {
            numeric_chars = numeric_chars + 1;
        }
    }

    //shrink:
    msg_len_str.truncate(numeric_chars);

    println!("receiving {} bytes", msg_len_str);

    /*
    let mut remaining_data = msg_len_str.parse::<i32>().unwrap();
    let mut accumulator: String = String::new();
    let mut r = [0u8; 8]; //8 byte buffer

    if remaining_data > 260
    //big message; write to file
    {

        //format file name
        // ../files/file1.py
        let start = command.rfind('/').unwrap() as usize;
        let end = command.rfind('.').unwrap() as usize;
        let mut file_name = String::from(&command[start + 1..end]);
        file_name.push_str(".json");
        println!("{}", &command);
        println!("{}", file_name);

        //create a file
        let mut file_buffer = BufWriter::new(File::create(file_name)?);

        while remaining_data != 0 {
            if remaining_data >= 8
            //slab >= 8 byte buffer
            {
                let slab = stream.read(&mut r);
                match slab {
                    Ok(n) => {
                        file_buffer.write(&mut r)?;
                        file_buffer.flush()?;
                        println!("wrote {} bytes to file", n);
                        remaining_data = remaining_data - n as i32;
                    }
                    _ => {}
                }
            } else
            //slab < 8 byte buffer
            {
                let array_limit = (remaining_data as i32) - 1;
                let slab = stream.read(&mut r);
                match slab {
                    Ok(n) => {
                        let mut r_slice = &r[0..(array_limit as usize + 1)]; // fixes underreading
                        // caused by not using
                        // subprocess call  on
                        // the server server
                        file_buffer.write(&mut r_slice)?;
                        file_buffer.flush()?;
                        println!("wrote {} bytes to file", n);
                        remaining_data = remaining_data - n as i32;
                    }
                    _ => {}
                }
            }
        }
        accumulator.push_str("response written to file");
    } else {
        //small message; receive as string
        while remaining_data != 0 {
            if remaining_data >= 8
            //slab >= 8 byte buffer
            {
                let slab = stream.read(&mut r);
                match slab {
                    Ok(n) => {
                        let r_slice = str::from_utf8(&mut r).unwrap(); //string slice
                        accumulator.push_str(r_slice);
                        println!("wrote {} bytes", n);
                        remaining_data = remaining_data - n as i32;
                    }
                    _ => {}
                }
            }
            /*
            option 1) receive and read a smaller buffer
            option 2) receive and read same buffer; truncate it to the smaller slab size

            since we cannot instantiate an array with a non-constant:
                e.g.: let mut r = [0u8; remainingData];
            it is better to just put the byte in the 8 byte buffer, and shrink it with
            .truncate() method before pushing to the String
            */
            else
            //slab < 8 byte buffer
            {
                let slab = stream.read(&mut r);
                match slab {
                    Ok(n) => {
                        let s_slice = str::from_utf8(&mut r).unwrap(); //string slice
                        let mut s_str = s_slice.to_string(); //convert slice to string
                        s_str.truncate(n);
                        accumulator.push_str(&s_str);
                        println!("wrote {} bytes", n);
                        remaining_data = remaining_data - n as i32;
                    }
                    _ => {}
                }
            }
        }
    }*/

    let response = ack_str;
    Ok(response)
}

fn terminal() -> io::Result<()> {
    let term = Term::stdout();
    term.write_line("Going to do some counting now")?;
    for x in 0..10 {
        if x != 0 {
            term.move_cursor_up(1)?;
        }
        term.write_line(&format!("Counting {}/10", style(x + 1).red()))?;
        thread::sleep(Duration::from_millis(200));
    }
    term.clear_last_lines(1)?;
    term.write_line("Done counting!")?;
    Ok(())
    
}

fn prefix() -> io::Result<()> {
    let term = Term::stdout();
    term.write_str("ftp> ")?;
    Ok(())
}

fn help(){
    println!("{}", style("Available FTP commands:").magenta());
    println!("{}\t\t-> {}", style("help").green(), style("show available commands").cyan());
    println!("{}\t-> {}", style("ls-local").green(), style("show local directory files").cyan());
    println!("{}\t-> {}", style("ls-remote").green(), style("show remote directory files").cyan());
    println!("{} {}\t-> {}", style("get").green(), style("\"filename\"").yellow(), style("download file from remote directory").cyan());
    println!("{} {}\t-> {}", style("put").green(), style("\"filename\"").yellow(), style("upload file to remote directory").cyan());
    println!("{}\t-> {}", style("exit/quit").green(), style("quit program").cyan());
}


fn local(){
    //let paths = fs::read_dir("./").unwrap();
    println!("{}", style("Local files (client/shared)").magenta());
    for entry in fs::read_dir("./src/shared").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            //clean path from file name:
            let mut fullpath = String::from(entry.path().to_string_lossy());
            let filename = String::from(str::replace(&fullpath, "./src/shared", ""));
            let trimmed = &filename[1..];

            let mut file = File::open(fullpath).unwrap();
            let file_size = file.metadata().unwrap().len();

            println!("{}  [{:?} bytes]", style(trimmed).green(), style(file_size).cyan());
        }
    }

    //for path in paths{
            //println!("{:?}", path.unwrap().path())
    //}
}

fn format_response(remote_list: &String) -> Vec<RemoteFileList>{
    //create vector of type RemoteFileList
    let mut remote_file_list: Vec<RemoteFileList> = Vec::new();
    //println!("{:?}", remote_list);

    //regex: /(\d+)(?= bytes\])/g

    lazy_static! {
        static ref file_size: Regex = Regex::new(r"(\d+)(?: bytes)").unwrap();
        //static ref file_name: Regex = Regex::new(r"(.*)(?:\s\[)(?:.*)(?: bytes\])").unwrap();
        static ref file_name: Regex = Regex::new(r"(.*)(?:\s\s\[.*\sbytes\])").unwrap();
    }
    for (i, cp) in file_name.captures_iter(remote_list).enumerate()
    {
        //first pass, push all names to vector
        //println!("file name: {} {}", i, &cp[1]);

        let mut current = RemoteFileList { f_name: String::from(""), f_size: String::from("") };
        current.f_name = String::from(&cp[1]);
        remote_file_list.push(current);
    }

    for (i, cap) in file_size.captures_iter(remote_list).enumerate() { 

        //second pass: edit all sizes according respective names
        //println!("{} {}", remote_file_list[i].f_name, &cap[1]);
        remote_file_list[i].f_size = String::from(&cap[1]);
    }
    remote_file_list
}

fn main() {
    //setup connection:
    //let mut stream = TcpStream::connect("127.0.0.1:5555")
    let mut stream = TcpStream::connect("127.0.0.1:5555") // try!(TcpStream::connect(HOST));
                                .expect("Couldn't connect to the server...");

    println!("connection to server successful");
    terminal().unwrap();
    println!("Type {} to see available commands:", style("help").magenta());
    loop {
        prefix().unwrap();
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let command = line.unwrap();

            if command.starts_with("get "){
                println!("user is trying to get a file");
                match check_cmd(&command, &mut stream) {
                    Ok(response) => println!("response: {}", response),
                    Err(err) => println!("An error occurred: {}", err),
                }
            }
            else if command.starts_with("put "){
                println!("user is trying to upload a file");
                match check_cmd(&command, &mut stream) {
                    Ok(response) => println!("response: {}", response),
                    Err(err) => println!("An error occurred: {}", err),
                }
            }
            else {
                match command.as_ref() {
                    "ls-remote" => {
                        match ls_remote(&command, &mut stream) {
                            Ok(response) => {
                                let formatted_response = format_response(&response);
                                //println!("{}\n{}", style("Remote files (server/shared)").magenta(), response);
                                println!("{}", style("Remote files (server/shared)").magenta());
                                for entry in formatted_response.iter() {
                                    //println!("{} {}", style(entry.f_name).green(), style(entry.f_size).cyan());
                                    println!("{}  [{} bytes]", style(&entry.f_name).green(), 
                                                               style(&entry.f_size).cyan());
                                }
                                
                            },
                            Err(err) => println!("An error occurred: {}", err),
                        }
                    },
                    "ls-local" => {
                        local();
                    },
                    "help" => {
                        help();
                    },
                    "exit" | "quit" => {
                        println!("user called exit");
                    },
                    _ => {
                        println!("invalid command!");
                    }
                }
            }
            break;
        }
    }
}