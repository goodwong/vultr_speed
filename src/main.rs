use async_std::sync::{Arc, Mutex};
use bytesize::ByteSize;
use chrono;
use reqwest;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://wa-us-ping.vultr.com/vultr.com.100MB.bin";
    let stats = Arc::new(Mutex::from(vec![]));

    // 启动下载
    let receiver = stats.clone();
    tokio::spawn(async move {
        download(url, receiver).await.unwrap();
    });

    // 输出
    let mut last_time = 0;
    loop {
        tokio::timer::delay_for(Duration::from_millis(1000)).await;

        let mut logs: Vec<(u128, usize)> = stats.lock().await.drain(0..).collect();
        // println!("logs: {:?}", logs.clone());
        let bytes = logs.iter().cloned().fold(0, |a, b| a + b.1);

        // 如果没有日志的话，end 就是当前时间戳
        // 有日志的话，就以最后一个日志的时间为准
        let end = match logs.pop() {
            Some((t, _)) => t,
            None => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        };
        let duration = end - last_time;
        // println!("{}, {}, {}, {}", last_time, end, duration, bytes);
        last_time = end;
        let speed = if duration > 0 {
            bytes / (duration as usize) * 1000
        } else {
            0
        };
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        print!("{:>8} ", timestamp);
        let column_width = 11;
        let max_speed = 4_000_000;
        print_speed(speed, max_speed, column_width);
    }
}

async fn download(
    url: &str,
    receiver: Arc<Mutex<Vec<(u128, usize)>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut res = reqwest::get(url).await?;

    while let Some(chunk) = res.chunk().await? {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let len = chunk.len();
        receiver.lock().await.push((now, len));
    }
    Ok(())
}

fn print_speed(speed: usize, max: usize, width: usize) {
    let output = format!("{:>9}/s", ByteSize::b(speed as u64).to_string());
    let speed_bar_len = usize::min(width, width * speed as usize / max);
    let (part1, part2) = if speed >= max {
        (
            format!("\x1B[7;31m{}\x1B[0m", &output[0..speed_bar_len]),
            format!("\x1B[31;m{}\x1B[0m", &output[speed_bar_len..]),
        )
    } else {
        (
            format!("\x1B[7m{}\x1B[0m", &output[0..speed_bar_len]),
            format!("\x1B[0m{}\x1B[0m", &output[speed_bar_len..]),
        )
    };
    println!("{}{}", part1, part2);
}
