mod core;
use core::{
    convert_buffer_into_file_audio, convert_file_audio_into_buffer, ffmpeg_blend_audios,
    ffmpeg_delay_audio,
};

fn main() {
    let audio_buffer_1 = convert_file_audio_into_buffer("audio-1.mp3").unwrap();
    let audio_buffer_2 = convert_file_audio_into_buffer("audio-2.mp3").unwrap();

    let bended_audio_buffer =
        ffmpeg_blend_audios(vec![audio_buffer_1.clone(), audio_buffer_2]).unwrap();
    convert_buffer_into_file_audio(bended_audio_buffer, "blended.mp3").unwrap();

    // let extended_audio_buffer = ffmpeg_delay_audio(audio_buffer_1, 1).unwrap();

    // convert_buffer_into_file_audio(extended_audio_buffer, "extended.mp3").unwrap();
}

// ffmpeg  -i audio-2.mp3 -i audio-3.mp3 -filter_complex amix=inputs=3:duration=first output.mp3

// ffmpeg -i audio-1.mp3 -i audio-2.mp3 -filter_complex amix=inputs=2:duration=first -f mp3 pipe:1 > mypipe

// cat mypipe > output.mp3

// ffmpeg -i audio-1.mp3 -af "adelay=7000|7000:all=true" extend.mp3

// 1 - Transformar audios inputs en un Vec<u8> tipo Buffer
// 2 - Ingresar inputs en el commando de ffmpeg. Seguir este ejemplo https://stackoverflow.com/questions/45899585/pipe-input-in-to-ffmpeg-stdin
// 3 - Transformar audio producido en Buffer a base64
