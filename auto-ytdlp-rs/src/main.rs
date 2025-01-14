use std::{fs, process::Command, thread};

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

    // Create chunks of URLs based on CPU count

    // Calculate chunk size using ceiling division to ensure all URLs are processed
    // For N urls and M cpus, we need ceiling(N/M) urls per chunk so that:
    // - Each CPU processes roughly equal number of URLs
    // - Some CPUs may process fewer URLs if N/M isn't even
    // - No URLs are left unprocessed
    // Example with 10 URLs, 3 CPUs:
    // - Chunk size = ceiling(10/3) = 4
    // - CPU 1: [URL1, URL2, URL3, URL4]
    // - CPU 2: [URL5, URL6, URL7, URL8]
    // - CPU 3: [URL9, URL10]
    let chunk_size = (urls.len() + num_cpus::get() - 1) / num_cpus::get();

    let url_chunks: Vec<Vec<String>> = urls
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Spawn threads for each chunk
    let mut handles = vec![];

    for chunk in url_chunks {
        let handle = thread::spawn(move || {
            for url in chunk {
                download_video(&url);
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

    println!("All downloads completed!");
}

fn download_video(url: &str) {
    println!("Downloading: {}", url);

    match Command::new("yt-dlp").arg(url).status() {
        Ok(status) => {
            if status.success() {
                println!("Successfully downloaded: {}", url);
            } else {
                eprintln!("Failed to download {}: Exit status: {}", url, status);
            }
        }
        Err(e) => eprintln!("Failed to execute yt-dlp for {}: {}", url, e),
    }
}
