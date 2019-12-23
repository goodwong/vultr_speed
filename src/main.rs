use async_std::sync::{Arc, Mutex};
use bytesize::ByteSize;
use chrono;
use reqwest;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio;

#[derive(Clone)]
struct PacketReport {
    index: u8,
    timestamp: u128,
    size: usize,
}

#[derive(Clone)]
struct Endpoint {
    name: String,
    url: String,
}

// todo 目前存在的问题：
// 1. 并行下载，表现为带宽冲突，需要改为轮候测试
// 2. 需要去除开始的前5秒
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoints = [
        Endpoint {
            name: "Frankfurt".to_string(),
            url: "https://fra-de-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Paris".to_string(),
            url: "https://par-fr-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Amsterdam".to_string(),
            url: "https://ams-cl-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "London".to_string(),
            url: "https://lon-gb-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "New York".to_string(),
            url: "https://nj-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Singapore".to_string(),
            url: "https://sgp-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Toronto".to_string(),
            url: "https://tor-ca-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Chicago".to_string(),
            url: "https://il-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Atlanta".to_string(),
            url: "https://ga-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Miami".to_string(),
            url: "https://fl-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Tokyo".to_string(),
            url: "https://hnd-jp-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Dallas".to_string(),
            url: "https://tx-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Seattle".to_string(),
            url: "https://wa-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Silicon Val".to_string(),
            url: "https://sjo-ca-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Los Angeles".to_string(),
            url: "https://lax-ca-us-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
        Endpoint {
            name: "Sydney".to_string(),
            url: "https://syd-au-ping.vultr.com/vultr.com.100MB.bin".to_string(),
        },
    ];
    let stats = Arc::new(Mutex::from(vec![]));

    // 启动下载
    for (i, endpoint) in endpoints.iter().enumerate() {
        let receiver = stats.clone();
        let endpoint = endpoint.clone();
        tokio::spawn(async move {
            download(i as u8, endpoint, receiver).await.unwrap();
        });
    }

    // 输出
    let column_width = 13;
    let max_speed = 1_000_000;

    let mut loop_index = 0;
    let mut last_time = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    loop {
        tokio::timer::delay_for(Duration::from_millis(1000)).await;

        // 表头
        if loop_index % 60 == 0 {
            println!();
            print!("{:8} ", "");
            for (_, endpoint) in endpoints.iter().enumerate() {
                print!("{:^width$}  ", endpoint.name, width = column_width);
            }
            println!();
        }
        loop_index += 1;

        // 收取日志
        let logs: Vec<PacketReport> = stats.lock().await.drain(0..).collect();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // 行头
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        print!("{:>8} ", timestamp);

        // 打印每一个数据中心速率
        for (i, _) in endpoints.iter().enumerate() {
            let logs = logs.iter().filter(|x| x.index == i as u8);
            let bytes = logs.clone().fold(0, |a, b| a + b.size);
            // 如果没有日志的话，end 就是当前时间戳
            // 有日志的话，就以最后一个日志的时间为准
            let end = {
                let last_receive_time = logs.clone().fold(0, |a, b| u128::max(a, b.timestamp));
                if last_receive_time > 0 {
                    last_receive_time
                } else {
                    now
                }
            };
            let duration = end - last_time[i];
            // println!("{}, {}, {}, {}", last_time, end, duration, bytes);
            last_time[i] = end;
            let speed = if duration > 0 {
                bytes / (duration as usize) * 1000
            } else {
                0
            };
            print_speed(speed, max_speed, column_width);
            print!("  ");
        }
        println!();
    }
}

async fn download(
    index: u8,
    endpoint: Endpoint,
    receiver: Arc<Mutex<Vec<PacketReport>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut res = reqwest::get(endpoint.url.as_str()).await?;

    while let Some(chunk) = res.chunk().await? {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let len = chunk.len();
        receiver.lock().await.push(PacketReport {
            index,
            timestamp: now,
            size: len,
        });
    }
    Ok(())
}

fn print_speed(speed: usize, max: usize, width: usize) {
    let output = format!(
        "{:<width$}",
        ByteSize::b(speed as u64).to_string() + "/s",
        width = width
    );
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
    print!("{}{}", part1, part2);
}
