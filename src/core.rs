use std::{
    fs::File,
    io::{BufReader, IoSlice, Read, Write},
    process::{Command, Stdio},
};

pub struct Audio {
    user_name: String,
    is_master_audio: bool,
    start_time: u32,
    buffer: Vec<u8>,
}

impl Audio {
    fn new(user_name: String, is_master_audio: bool, start_time: u32, buffer: Vec<u8>) -> Audio {
        Audio {
            user_name,
            is_master_audio,
            start_time,
            buffer,
        }
    }
}

pub struct Session {
    audios: Vec<Audio>,
}

impl Session {
    fn new_audio(&mut self, audio: Audio) {
        self.audios.push(audio)
    }
}

pub fn ffmpeg_delay_audio(audio_buffer: Vec<u8>, delay: u32) -> std::io::Result<Vec<u8>> {
    let mut child = Command::new("/usr/bin/ffmpeg")
        .args(&[
            "-f",
            "mp3",
            "-i",
            "pipe:0",
            "-af",
            format!("adelay={delay}|{delay}:all=true", delay = delay * 1000).as_str(),
            "-f",
            "mp3",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut child_stdin = child.stdin.take().unwrap();

    std::thread::spawn(move || {
        child_stdin.write(&audio_buffer).unwrap();
    });

    let output = child.wait_with_output().unwrap();

    Ok(output.stdout)
}

pub fn ffmpeg_blend_audios(audio_buffers: Vec<Vec<u8>>) -> std::io::Result<Vec<u8>> {
    let mut child = Command::new("/usr/bin/ffmpeg")
        .args(&[
            "-f",
            "mp3",
            "-i",
            "pipe:0",
            "-f",
            "mp3",
            "-i",
            "pipe:0",
            "-filter_complex",
            "amix=inputs=2:duration=first",
            "-f",
            "mp3",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // let audio_buffers = audio_buffers
    //     .iter()
    //     .map(|item| IoSlice::new(item))
    //     .collect::<Vec<IoSlice>>();

    let mut child_stdin = child.stdin.take().unwrap();

    std::thread::spawn(move || {
        child_stdin
            .write_vectored(&vec![
                IoSlice::new(&audio_buffers[0]),
                IoSlice::new(&audio_buffers[1]),
            ])
            .unwrap();
    });

    let output = child.wait_with_output().unwrap();

    Ok(output.stdout)
}

///
///
///////// temporal functions
///
///

pub fn convert_file_audio_into_buffer(audio_name: &str) -> std::io::Result<Vec<u8>> {
    let f = File::open(audio_name).unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = vec![];
    reader.read_to_end(&mut buffer).unwrap();

    Ok(buffer)
}

pub fn convert_buffer_into_file_audio(buffer: Vec<u8>, name: &str) -> std::io::Result<()> {
    let mut file = File::create(name).unwrap();
    file.write_all(&buffer).unwrap();
    Ok(())
}

pub fn test_stdin() {
    let mut head_cmd = Command::new("/usr/bin/head");
    head_cmd.args(&["-n 1"]);
    head_cmd.stdin(Stdio::piped());
    head_cmd.stdout(Stdio::piped());

    let input_data = b"test1\ntest2";

    let mut proc_handle = head_cmd.spawn().unwrap();
    let mut stdin_handle = proc_handle.stdin.take().unwrap();

    _ = stdin_handle.write_all(input_data);
    // proc_handle.wait().unwrap();
    let mut output_buffer = String::new();
    proc_handle
        .stdout
        .unwrap()
        .read_to_string(&mut output_buffer)
        .unwrap();

    println!("Result: {:?}", output_buffer)
}

pub fn test_stdin2(audio_buffers: Vec<u8>) -> std::io::Result<Vec<u8>> {
    let mut head_cmd = Command::new("/usr/bin/ffmpeg");
    head_cmd.args(&["-f", "mp3", "-i", "pipe:0", "-f", "wav", "pipe:1"]);
    head_cmd.stdin(Stdio::piped());
    head_cmd.stdout(Stdio::piped());

    let mut proc_handle = head_cmd.spawn().unwrap();

    let mut stdin_handle = proc_handle.stdin.take().unwrap();
    std::thread::spawn(move || {
        stdin_handle
            .write_vectored(&vec![IoSlice::new(&audio_buffers)])
            .expect("Failed to write to stdin");
    });

    // proc_handle.wait().unwrap();
    let output = proc_handle.wait_with_output().unwrap();

    Ok(output.stdout)
}
