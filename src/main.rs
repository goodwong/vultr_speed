use async_std::sync::{Arc, Mutex};
use bytesize::ByteSize;
use chrono;
use reqwest;
use std::io::{stdout, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use termion;
use tokio;

#[derive(Clone)]
struct PacketReport {
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
    let _guard = termion::init(); // for windows

    // 清屏
    print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));

    let endpoints = [
        Endpoint {
            name: "Frankfurt".to_string(),
            url: "https://fra-de-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Paris".to_string(),
            url: "https://par-fr-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        //Endpoint {
        //    name: "Amsterdam".to_string(),
        //    url: "https://ams-cl-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        //},
        Endpoint {
            name: "London".to_string(),
            url: "https://lon-gb-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "New York".to_string(),
            url: "https://nj-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Singapore".to_string(),
            url: "https://sgp-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Toronto".to_string(),
            url: "https://tor-ca-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Chicago".to_string(),
            url: "https://il-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Atlanta".to_string(),
            url: "https://ga-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Miami".to_string(),
            url: "https://fl-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Tokyo".to_string(),
            url: "https://hnd-jp-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Dallas".to_string(),
            url: "https://tx-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Seattle".to_string(),
            url: "https://wa-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Silicon Val".to_string(),
            url: "https://sjo-ca-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Los Angeles".to_string(),
            url: "https://lax-ca-us-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
        Endpoint {
            name: "Sydney".to_string(),
            url: "https://syd-au-ping.vultr.com/vultr.com.1000MB.bin".to_string(),
        },
    ];
    let logs = Arc::new(Mutex::from(vec![]));

    let header_width: usize = 10;
    let column_width: usize = 12;
    let loop_per_block = 60;

    let mut loop_index: i32 = 0;
    loop {
        // 表头
        if loop_index % loop_per_block == 0 {
            println!();
            print!("{:10}", "");
            for (i, endpoint) in endpoints.iter().enumerate() {
                let x = (header_width + i * (column_width + 1) + 1) as u16;
                let y = (loop_index + (loop_index / loop_per_block) * 2 + 2) as u16;
                print!(
                    "{}{:^width$} ",
                    termion::cursor::Goto(x, y),
                    endpoint.name,
                    width = column_width
                );
            }
            println!();
        }

        // 行头
        let y = (loop_index + (loop_index / loop_per_block) * 2 + 3) as u16;
        if loop_index % 12 == 0 {
            let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
            print!("{}{:10}", termion::cursor::Goto(1, y), timestamp);
        } else {
            print!("{}{:10}", termion::cursor::Goto(1, y), "");
        }
        stdout().flush()?;

        // 显示每个数据中心
        for (j, endpoint) in endpoints.iter().enumerate() {
            // 启动下载
            {
                let receiver = logs.clone();
                let endpoint = endpoint.clone();
                tokio::spawn(async move {
                    download(endpoint, receiver).await.unwrap_or(());
                });
            }
            // 去掉开头8秒数据
            tokio::timer::delay_for(Duration::from_millis(8_000)).await;

            // 动态输出每秒速度
            let x = (header_width + j * (column_width + 1) + 1) as u16;
            let mut bytes_in_10s = 0;
            for _ in 0..10 {
                // 等待1秒
                tokio::timer::delay_for(Duration::from_millis(1_000)).await;
                // 收取日志
                // todo 此处需要增加时间范围过滤，更为精确。
                let logs: Vec<PacketReport> = logs.lock().await.drain(0..).collect();

                let bytes = logs.iter().fold(0, |a, b| a + b.size);
                let speed = bytes / 1; // 每秒刷新

                print!("{}", termion::cursor::Goto(x, y));
                print_speed(speed, column_width);
                print!("{}", termion::cursor::Goto(x, y));
                stdout().flush()?;

                bytes_in_10s += bytes;
            }

            // 最终输出平均速度
            let average_speed = bytes_in_10s / 10;
            tokio::timer::delay_for(Duration::from_millis(2_000)).await;
            print!("{}", termion::cursor::Goto(x, y));
            print_speed(average_speed, column_width);

            stdout().flush()?;
        }
        println!();
        loop_index += 1;
    }
}

async fn download(
    endpoint: Endpoint,
    receiver: Arc<Mutex<Vec<PacketReport>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 去掉开头10秒（含连接时间）
    // 只取中间10秒
    let start = now();

    let mut res = reqwest::get(endpoint.url.as_str()).await?;
    while let Some(chunk) = res.chunk().await? {
        let timestamp = now();
        let size = chunk.len();
        if timestamp - start < 8_000 {
            continue;
        }

        if timestamp - start >= 18_000 {
            break;
        }
        receiver.lock().await.push(PacketReport { timestamp, size });
    }
    Ok(())
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn print_speed(speed: usize, column_width: usize) {
    // 查看所有颜色命令：for x in {0..8}; do for i in {30..37}; do for a in {40..47}; do echo -ne "\e[$x;$i;$a""m\\\e[$x;$i;$a""m\e[0;37;40m "; done; echo; done; done; echo ""
    // 参见：https://askubuntu.com/questions/27314/script-to-display-all-terminal-colors
    //
    // 速率UI：
    //    0     ~    100kB   0格  100/ 白色段（\e[0;30;47m）
    //    100kB ~    200kB   1格  100/
    //    200kB ~    400kB   2格  200/
    //    400kB ~    600kB   3格  200/
    //    600kB ~  1,000kB   4格  400/
    //  1,000kB ~  1,400kB   5格  400/
    //  1,400kB ~  2,200kB   6格  800/
    //  2,200kB ~  3,000kB   7格  800/ 紫色段（\e[0;30;45m）
    //  3,000kB ~  4,600kB   8格 1600/
    //  4,600kB ~  6,200kB   9格 1600/
    //  6,200kB ~  9,400kB  10格 3200/ 红色段（\e[0;30;41m）
    //  9,400kB ~ 12,600kB  11格 3200/
    // 12,600kB ~ 19,000kB  12格 6400/
    //

    let output = format!(
        "{:<width$}",
        ByteSize::b(speed as u64).to_string() + "/s",
        width = column_width
    );

    let white = "\x1B[0;30;47m";
    let reset = "\x1B[0m";
    let magenta = "\x1B[0;30;45m";
    let red = "\x1B[0;30;41m";

    let bar = match speed {
        // 白色段
        0..=100_000 => format!("{}", &output),
        100_000..=200_000 => format!("{}{}{}{}", white, &output[0..1], reset, &output[1..]),
        200_000..=400_000 => format!("{}{}{}{}", white, &output[0..2], reset, &output[2..]),
        400_000..=600_000 => format!("{}{}{}{}", white, &output[0..3], reset, &output[3..]),
        600_000..=1_000_000 => format!("{}{}{}{}", white, &output[0..4], reset, &output[4..]),
        1_000_000..=1_400_000 => format!("{}{}{}{}", white, &output[0..5], reset, &output[5..]),
        1_400_000..=2_200_000 => format!("{}{}{}{}", white, &output[0..6], reset, &output[6..]),
        // 紫色段
        2_200_000..=3_000_000 => format!(
            "{}{}{}{}{}{}",
            white,
            &output[0..6],
            magenta,
            &output[6..7],
            reset,
            &output[7..]
        ),
        3_000_000..=4_600_000 => format!(
            "{}{}{}{}{}{}",
            white,
            &output[0..6],
            magenta,
            &output[6..8],
            reset,
            &output[8..]
        ),
        4_600_000..=6_200_000 => format!(
            "{}{}{}{}{}{}",
            white,
            &output[0..6],
            magenta,
            &output[6..9],
            reset,
            &output[9..]
        ),
        // 红色段
        6_200_000..=9_400_000 => format!(
            "{}{}{}{}{}{}{}{}",
            white,
            &output[0..6],
            magenta,
            &output[6..9],
            red,
            &output[9..10],
            reset,
            &output[10..]
        ),
        9_400_000..=12_600_000 => format!(
            "{}{}{}{}{}{}{}{}",
            white,
            &output[0..6],
            magenta,
            &output[6..9],
            red,
            &output[9..11],
            reset,
            &output[11..]
        ),
        12_600_000..=19_000_000 => format!(
            "{}{}{}{}{}{}{}",
            white,
            &output[0..6],
            magenta,
            &output[6..9],
            red,
            &output[9..12],
            reset
        ),
        _ => format!("{}{}{}", red, &output, reset),
    };
    print!("{}", bar);
}
