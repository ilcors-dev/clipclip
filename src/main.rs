use chrono::Local;
use enigo::{Enigo, KeyboardControllable};
use porcupine::{Porcupine, PorcupineBuilder};
use pv_recorder::RecorderBuilder;
use std::sync::atomic::{AtomicBool, Ordering};

static LISTENING: AtomicBool = AtomicBool::new(false);

#[macro_use]
extern crate dotenv_codegen;

/**
 * Run the porcupine voice recognition engine to listen for the wake word.
 */
fn run(
    access_key: String,
    input_device_index: i32,
    keyword_paths: Vec<String>,
    model_path: String,
) {
    let mut enigo = Enigo::new();

    let porcupine: Porcupine =
        PorcupineBuilder::new_with_keyword_paths(access_key, keyword_paths.as_slice())
            .model_path(model_path)
            .init()
            .expect("Unable to create Porcupine");

    let recorder = RecorderBuilder::new()
        .device_index(input_device_index)
        .frame_length(porcupine.frame_length() as i32)
        .init()
        .expect("Failed to initialize pvrecorder");

    recorder.start().expect("Failed to start audio recording");

    LISTENING.store(true, Ordering::SeqCst);
    ctrlc::set_handler(|| {
        LISTENING.store(false, Ordering::SeqCst);
    })
    .expect("Unable to setup signal handler");

    println!("Listening for wake words...");

    while LISTENING.load(Ordering::SeqCst) {
        let mut pcm = vec![0; recorder.frame_length()];
        recorder.read(&mut pcm).expect("Failed to read audio frame");

        let keyword_index = porcupine.process(&pcm).unwrap();
        if keyword_index >= 0 {
            println!("[{}] clippo!", Local::now().format("%F %T"));
            enigo.key_down(enigo::Key::Alt);
            enigo.key_click(enigo::Key::F10);
            enigo.key_up(enigo::Key::Alt);
        }
    }

    println!("\nStopping...");
    recorder.stop().expect("Failed to stop audio recording");
}

fn show_audio_devices() -> Vec<i32> {
    let mut device_indices = Vec::<i32>::new();

    let audio_devices = RecorderBuilder::new()
        .init()
        .expect("Failed to initialize pvrecorder")
        .get_audio_devices();

    match audio_devices {
        Ok(audio_devices) => {
            for (idx, device) in audio_devices.iter().enumerate() {
                device_indices.push(idx as i32);
                println!("index: {}, device name: {:?}", idx, device);
            }
        }
        Err(err) => panic!("Failed to get audio devices: {}", err),
    };

    return device_indices;
}

fn select_input_source() -> i32 {
    let sources = show_audio_devices();

    print!("Select input source by index: ");

    let mut input = String::new();

    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let trimmed = input.trim();

    match trimmed.parse::<i32>() {
        Ok(i) => i,
        Err(..) => panic!("This was not an integer: {}", trimmed),
    };

    let input_source = input.trim().parse::<i32>().unwrap();

    if !sources.contains(&input_source) {
        panic!(
            "Invalid input source, allowed sources indexes are: {:?}",
            sources
        );
    }

    return input_source;
}

/**
 * Returns the path to the keyword file based on the user's os.
 */
fn determine_ppn_by_os(curdir: String) -> String {
    let platform = std::env::consts::OS;
    let keyword_path_mac_os = format!("{}/wake_words/fai-la-clip_it_mac_v2_1_0.ppn", curdir);
    let keyword_path_windows = format!("{}/wake_words/fai-la-clip_it_windows_v2_1_0.ppn", curdir);

    let ppn = match platform {
        "macos" => keyword_path_mac_os,
        "windows" => keyword_path_windows,
        _ => panic!("Unsupported platform"),
    };

    return ppn.to_string();
}

fn main() {
    let binding = std::env::current_dir().unwrap();
    let curdir = binding.display();
    let model_path = format!("{}/wake_words/porcupine_params_it.pv", curdir);

    let input_source = select_input_source();
    let mut keyword_path = Vec::<String>::new();
    keyword_path.push(determine_ppn_by_os(curdir.to_string()));

    let access_key = dotenv!("PORCUPINE_ACCESS_KEY").to_owned();

    run(access_key, input_source, keyword_path, model_path);
}
