use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fs,
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

fn main() {
    // Read URLs from file
    let content = match fs::read_to_string("./links.txt") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to read link.txt: {}", e);
            return;
        }
    };

    // Get list of URLs, filtering out empty lines
    let urls: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(String::from)
        .collect();

    if urls.is_empty() {
        println!("No URLs found in link.txt");
        return;
    }

    // Create progress bar
    let total_urls = urls.len();
    let pb = ProgressBar::new(total_urls as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    // Wrap progress bar in Arc<Mutex> so it can be shared between threads
    let pb = Arc::new(Mutex::new(pb));

    // Calculate chunk size using ceiling division to ensure all URLs are processed
    // For N urls and M cpus, we need ceiling(N/M) urls per chunk so that:
    // - Each CPU processes roughly equal number of URLs
    // - Some CPUs may process fewer URLs if N/M isn't even
    // - No URLs are left unprocessed
    // Example with 10 URLs, 3 CPUs:
    // - Chunk size = ceiling(10/3) = 4
    // - CPU 1: [URL1-4]
    // - CPU 2: [URL5-8]
    // - CPU 3: [URL9-10]
    let chunk_size = (urls.len() + num_cpus::get() - 1) / num_cpus::get();
    let url_chunks: Vec<Vec<String>> = urls
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Spawn threads for each chunk
    let mut handles = vec![];

    for chunk in url_chunks {
        let pb = Arc::clone(&pb);
        let handle = thread::spawn(move || {
            for url in chunk {
                download_video(&url);
                // Increment progress bar
                if let Ok(pb) = pb.lock() {
                    pb.inc(1);
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all downloads to complete
    for handle in handles {
        if let Err(e) = handle.join() {
            eprintln!("A thread panicked: {:?}", e);
        }
    }

    // Finish progress bar
    if let Ok(pb) = pb.lock() {
        pb.finish_with_message("All downloads completed!");
    };
}

fn download_video(url: &str) {
    match Command::new("yt-dlp")
        .arg(url)
        .stdout(std::process::Stdio::null())  // Hide stdout
        .stderr(std::process::Stdio::piped())  // Keep stderr for error reporting
        .output()  // Use output() instead of status() to capture stderr
    {
        Ok(output) => {
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                eprintln!("Failed to download {}: {}", url, error);
            }
        }
        Err(e) => eprintln!("Failed to execute yt-dlp for {}: {}", url, e),
    }
}
