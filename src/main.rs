use bytesize::ByteSize;
use reqwest;
//use std::io::{stdout, Write};
use std::time::Instant;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://wa-us-ping.vultr.com/vultr.com.100MB.bin";
    reqwest_download(url).await
}

async fn reqwest_download(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut res = reqwest::get(url).await?;

    let time_start = Instant::now();
    let mut time_previous = 0;
    let mut downloaded = 0;
    let mut bytes_batch = 0;
    let content_length = res.content_length();

    // todo 其实应该独立一个线程每秒计算的，
    // todo 否则网络卡死了，瞬时速度不会归零
    // todo 暂时这样做
    while let Some(chunk) = res.chunk().await? {
        let now = time_start.elapsed().as_secs();

        bytes_batch += chunk.len() as u64;
        if now > time_previous {
            // 计算
            downloaded += bytes_batch;
            let speed_current = bytes_batch / (now - time_previous);
            let speed_average = downloaded / now;
            // 重置计数器
            bytes_batch = 0;
            time_previous = now;

            // 输出
            //print!("\r");
            print!(
                "Speed: {:>9}/{:<9}; ",
                ByteSize::b(speed_current).to_string(),
                ByteSize::b(speed_average).to_string()
            );
            print!("Progress: {:>9}", ByteSize::b(downloaded).to_string());
            match content_length {
                Some(lenth) => {
                    print!("/{:<9}", ByteSize::b(lenth).to_string());
                }
                None => {
                    print!("/-        ");
                }
            };
            //stdout().flush().unwrap();

            println!(
                " {:<30}",
                "█".repeat(30 * speed_current as usize / 2_000_000)
            );
        }
    }
    println!("\nDone!");
    Ok(())
}
