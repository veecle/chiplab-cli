use std::io::{IsTerminal, Write};

use terminal_colorsaurus::{QueryOptions, ThemeMode};

const BANNERS: &[&[&str]] = &[
    &[
        r"   $$$$$$$$$$$$\   $$$$\             $$$$\                   $$$$\                   $$$$\              ",
        r"  $$$$$$$$$$$$$$\  $$$$ |            $$$$ |                  $$$$ |                  $$$$ |             ",
        r" $$$$$ _____$$$$$\ $$$$ |            \____|                  $$$$ |                  $$$$ |             ",
        r" $$$$ /     \$$$$ |$$$$ |                                    $$$$ |                  $$$$ |             ",
        r" $$$$ |      \____|$$$$$$$$$$$$$$\   $$$$\   $$$$$$$$$$$$\   $$$$ |  $$$$$$$$$$$$\   $$$$$$$$$$$$$$\    ",
        r" $$$$ |            $$$$$$$$$$$$$$$\  $$$$ | $$$$$$$$$$$$$$\  $$$$ |  $$$$$$$$$$$$$\  $$$$$$$$$$$$$$$\   ",
        r" $$$$ |            $$$$  ______$$$$\ $$$$ |$$$$$ _____$$$$$\ $$$$ |  \________$$$$$\ $$$$  _____$$$$$\  ",
        r" $$$$ |            $$$$ |      $$$$ |$$$$ |$$$$ /      $$$$ |$$$$ |            $$$$ |$$$$ |      $$$$ | ",
        r" $$$$ |            $$$$ |      $$$$ |$$$$ |$$$$ |      $$$$ |$$$$ |  $$$$$$$$$$$$$$ |$$$$ |      $$$$ | ",
        r" $$$$ |            $$$$ |      $$$$ |$$$$ |$$$$ |      $$$$ |$$$$ | $$$$$$$$$$$$$$$ |$$$$ |      $$$$ | ",
        r" $$$$ |      $$$$\ $$$$ |      $$$$ |$$$$ |$$$$ |      $$$$ |$$$$ |$$$$$ ______$$$$ |$$$$ |      $$$$ | ",
        r" $$$$$\     $$$$$ |$$$$ |      $$$$ |$$$$ |$$$$ |     $$$$$ |$$$$ |$$$$$       $$$$ |$$$$ |     $$$$$ | ",
        r" \$$$$$$$$$$$$$$  |$$$$ |      $$$$ |$$$$ |$$$$$$$$$$$$$$$  |$$$$ |\$$$$$$$$$$$$$$$ |$$$$$$$$$$$$$$$  | ",
        r"  \$$$$$$$$$$$$  / $$$$ |      $$$$ |$$$$ |$$$$$$$$$$$$$$  / $$$$ | \$$$$$$$$$$$$$$ |$$$$$$$$$$$$$$  /  ",
        r"   \____________/  \____|      \____|\____|$$$$  _________/  \____|  \______________|\______________/   ",
        r"                                           $$$$ |                                                       ",
        r"                                           $$$$ |                                                       ",
        r"                                           $$$$ |                                                       ",
        r"                                           \____|                                                       ",
    ],
    &[
        r"   CCCCCCCCCCCC\   hhhh\             iiii\                   llll\                   bbbb\              ",
        r"  CCCCCCCCCCCCCC\  hhhh |            iiii |                  llll |                  bbbb |             ",
        r" CCCCC _____CCCCC\ hhhh |            \____|                  llll |                  bbbb |             ",
        r" CCCC /     \CCCC |hhhh |                                    llll |                  bbbb |             ",
        r" CCCC |      \____|hhhhhhhhhhhhhh\   iiii\   pppppppppppp\   llll |  aaaaaaaaaaaa\   bbbbbbbbbbbbbb\    ",
        r" CCCC |            hhhhhhhhhhhhhhh\  iiii | pppppppppppppp\  llll |  aaaaaaaaaaaaa\  bbbbbbbbbbbbbbb\   ",
        r" CCCC |            hhhh  ______hhhh\ iiii |ppppp _____ppppp\ llll |  \________aaaaa\ bbbb  _____bbbbb\  ",
        r" CCCC |            hhhh |      hhhh |iiii |pppp /      pppp |llll |            aaaa |bbbb |      bbbb | ",
        r" CCCC |            hhhh |      hhhh |iiii |pppp |      pppp |llll |  aaaaaaaaaaaaaa |bbbb |      bbbb | ",
        r" CCCC |            hhhh |      hhhh |iiii |pppp |      pppp |llll | aaaaaaaaaaaaaaa |bbbb |      bbbb | ",
        r" CCCC |      CCCC\ hhhh |      hhhh |iiii |pppp |      pppp |llll |aaaaa ______aaaa |bbbb |      bbbb | ",
        r" CCCCC\     CCCCC |hhhh |      hhhh |iiii |pppp |     ppppp |llll |aaaaa       aaaa |bbbb |     bbbbb | ",
        r" \CCCCCCCCCCCCCC  |hhhh |      hhhh |iiii |ppppppppppppppp  |llll |\aaaaaaaaaaaaaaa |bbbbbbbbbbbbbbb  | ",
        r"  \CCCCCCCCCCCC  / hhhh |      hhhh |iiii |pppppppppppppp  / llll | \aaaaaaaaaaaaaa |bbbbbbbbbbbbbb  /  ",
        r"   \____________/  \____|      \____|\____|pppp  _________/  \____|  \______________|\______________/   ",
        r"                                           pppp |                                                       ",
        r"                                           pppp |                                                       ",
        r"                                           pppp |                                                       ",
        r"                                           \____|                                                       ",
    ],
    &[
        " ████████████████╲ ████╲             ████╲                   ████╲                   ████╲             ",
        " ████████████████▕▏████▕▏            ████▕▏                  ████▕▏                  ████▕▏            ",
        " ████╲       ████▕▏████▕▏            ╲▁▁▁🭿▏                  ████▕▏                  ████▕▏            ",
        " ████▕🭽▔▔▔▔▔▔████▕▏████▕▏                                    ████▕▏                  ████▕▏            ",
        " ████▕▏      ╲▁▁▁🭿▏████████████████╲ ████╲ ████████████████╲ ████▕▏  ██████████████╲ ████████████████╲ ",
        " ████▕▏            ████████████████▕▏████▕▏████████████████▕▏████▕▏  ██████████████▕▏████████████████▕▏",
        " ████▕▏            ████╲       ████▕▏████▕▏████╲       ████▕▏████▕▏  ╲         ████▕▏████╲       ████▕▏",
        " ████▕▏            ████▕🭽▔▔▔▔▔▔████▕▏████▕▏████▕🭽▔▔▔▔▔▔████▕▏████▕▏   ▔▔▔▔▔▔▔▔▔████▕▏████▕🭽▔▔▔▔▔▔████▕▏",
        " ████▕▏            ████▕▏      ████▕▏████▕▏████▕▏      ████▕▏████▕▏████████████████▕▏████▕▏      ████▕▏",
        " ████▕▏            ████▕▏      ████▕▏████▕▏████▕▏      ████▕▏████▕▏████████████████▕▏████▕▏      ████▕▏",
        " ████▕▏      ████╲ ████▕▏      ████▕▏████▕▏████▕▏      ████▕▏████▕▏████╲       ████▕▏████▕▏      ████▕▏",
        " ████▕▏      ████▕▏████▕▏      ████▕▏████▕▏████▕▏      ████▕▏████▕▏████▕🭽▔▔▔▔▔▔████▕▏████▕▏      ████▕▏",
        " ████████████████▕▏████▕▏      ████▕▏████▕▏████████████████▕▏████▕▏████████████████▕▏████████████████▕▏",
        " ████████████████▕▏████▕▏      ████▕▏████▕▏████████████████▕▏████▕▏████████████████▕▏████████████████▕▏",
        " ╲▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁🭿▏╲▁▁▁🭿▏      ╲▁▁▁🭿▏╲▁▁▁🭿▏████╲▁▁▁▁▁▁▁▁▁▁▁🭿▏╲▁▁▁🭿▏╲▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁🭿▏╲▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁🭿▏",
        "                                           ████▕▏                                                      ",
        "                                           ████▕▏                                                      ",
        "                                           ████▕▏                                                      ",
        "                                           ╲▁▁▁🭿▏                                                      ",
    ],
    &[
        " ████████████████🮟 ████🮟             ████🮟                   ████🮟                   ████🮟             ",
        " ████████████████▒ ████▒             ████▒                   ████▒                   ████▒             ",
        " ████▒▒▒▒▒▒▒▒████▒ ████▒             🮝▒▒▒▒                   ████▒                   ████▒             ",
        " ████▒       ████▒ ████▒                                     ████▒                   ████▒             ",
        " ████▒       🮝▒▒▒▒ ████████████████🮟 ████🮟 ████████████████🮟 ████▒   ██████████████🮟 ████████████████🮟 ",
        " ████▒             ████████████████▒ ████▒ ████████████████▒ ████▒   ██████████████▒ ████████████████▒ ",
        " ████▒             ████▒▒▒▒▒▒▒▒████▒ ████▒ ████▒▒▒▒▒▒▒▒████▒ ████▒   🮝▒▒▒▒▒▒▒▒▒████▒ ████▒▒▒▒▒▒▒▒████▒ ",
        " ████▒             ████▒       ████▒ ████▒ ████▒       ████▒ ████▒             ████▒ ████▒       ████▒ ",
        " ████▒             ████▒       ████▒ ████▒ ████▒       ████▒ ████▒ ████████████████▒ ████▒       ████▒ ",
        " ████▒             ████▒       ████▒ ████▒ ████▒       ████▒ ████▒ ████████████████▒ ████▒       ████▒ ",
        " ████▒       ████🮟 ████▒       ████▒ ████▒ ████▒       ████▒ ████▒ ████▒▒▒▒▒▒▒▒████▒ ████▒       ████▒ ",
        " ████▒       ████▒ ████▒       ████▒ ████▒ ████▒       ████▒ ████▒ ████        ████▒ ████▒       ████▒ ",
        " ████████████████▒ ████▒       ████▒ ████▒ ████████████████▒ ████▒ ████████████████▒ ████████████████▒ ",
        " ████████████████▒ ████▒       ████▒ ████▒ ████████████████▒ ████▒ ████████████████▒ ████████████████▒ ",
        " 🮝▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ 🮝▒▒▒▒       🮝▒▒▒▒ 🮝▒▒▒▒ ████▒▒▒▒▒▒▒▒▒▒▒▒▒ 🮝▒▒▒▒ 🮝▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ 🮝▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ ",
        "                                           ████▒                                                       ",
        "                                           ████▒                                                       ",
        "                                           ████▒                                                       ",
        "                                           🮝▒▒▒▒                                                       ",
    ],
    &[
        " ████████████████  ████              ████                    ████                    ████             ",
        " ████████████████  ████              ████                    ████                    ████             ",
        " ████        ████  ████                                      ████                    ████             ",
        " ████        ████  ████                                      ████                    ████             ",
        " ████              ████████████████  ████  ████████████████  ████    ██████████████  ████████████████ ",
        " ████              ████████████████  ████  ████████████████  ████    ██████████████  ████████████████ ",
        " ████              ████        ████  ████  ████        ████  ████              ████  ████        ████ ",
        " ████              ████        ████  ████  ████        ████  ████              ████  ████        ████ ",
        " ████              ████        ████  ████  ████        ████  ████  ████████████████  ████        ████ ",
        " ████              ████        ████  ████  ████        ████  ████  ████████████████  ████        ████ ",
        " ████        ████  ████        ████  ████  ████        ████  ████  ████        ████  ████        ████ ",
        " ████        ████  ████        ████  ████  ████        ████  ████  ████        ████  ████        ████ ",
        " ████████████████  ████        ████  ████  ████████████████  ████  ████████████████  ████████████████ ",
        " ████████████████  ████        ████  ████  ████████████████  ████  ████████████████  ████████████████ ",
        "                                           ████                                                       ",
        "                                           ████                                                       ",
        "                                           ████                                                       ",
        "                                           ████                                                       ",
    ],
    &[
        "▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄            ▄▄▄▄▄▄                  ▄▄▄▄▄▄                  ▄▄▄▄▄▄            ",
        "█                ██    █            █    █                  █    █                  █    █            ",
        "█                ██    █            █    █                  █    █                  █    █            ",
        "█    █▀▀▀▀▀▀█    ██    █            ▀▀▀▀▀▀                  █    █                  █    █            ",
        "█    █      █    ██    █▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█    █  ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█    █▄▄▄▄▄▄▄▄▄▄▄▄",
        "█    █      ▀▀▀▀▀▀█                ██    ██                ██    █  █              ██                █",
        "█    █            █                ██    ██                ██    █  █              ██                █",
        "█    █            █    █▀▀▀▀▀▀█    ██    ██    █▀▀▀▀▀▀█    ██    █  ▀▀▀▀▀▀▀▀▀▀█    ██    █▀▀▀▀▀▀█    █",
        "█    █            █    █      █    ██    ██    █      █    ██    █▄▄▄▄▄▄▄▄▄▄▄▄█    ██    █      █    █",
        "█    █            █    █      █    ██    ██    █      █    ██    ██                ██    █      █    █",
        "█    █      ▄▄▄▄▄▄█    █      █    ██    ██    █      █    ██    ██                ██    █      █    █",
        "█    █      █    ██    █      █    ██    ██    █      █    ██    ██    █▀▀▀▀▀▀█    ██    █      █    █",
        "█    █▄▄▄▄▄▄█    ██    █      █    ██    ██    █▄▄▄▄▄▄█    ██    ██    █▄▄▄▄▄▄█    ██    █▄▄▄▄▄▄█    █",
        "█                ██    █      █    ██    ██                ██    ██                ██                █",
        "█                ██    █      █    ██    ██                ██    ██                ██                █",
        "▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀      ▀▀▀▀▀▀▀▀▀▀▀▀█    █▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀",
        "                                          █    █                                                      ",
        "                                          █    █                                                      ",
        "                                          █    █                                                      ",
        "                                          ▀▀▀▀▀▀                                                      ",
    ],
];

const DARK_STOPS: &[[u8; 3]] = &[
    [0xc4, 0x5c, 0x00],
    [0xd9, 0x70, 0x22],
    [0xe8, 0x89, 0x3a],
    [0xc8, 0xa9, 0x6e],
    [0xa8, 0xb8, 0x7a],
    [0xc8, 0xd4, 0xa8],
    [0xe8, 0xed, 0xe0],
    [0xee, 0xe9, 0xdf],
];

const LIGHT_STOPS: &[[u8; 3]] = &[
    [0xc4, 0x5c, 0x00],
    [0xb8, 0x53, 0x00],
    [0xa8, 0x5a, 0x14],
    [0x8a, 0x6a, 0x2e],
    [0x5f, 0x7a, 0x3a],
    [0x4a, 0x66, 0x38],
    [0x36, 0x4a, 0x2e],
    [0x2b, 0x33, 0x26],
];

const ANGLE: f64 = 135.0;

/// Terminal cells are roughly twice as tall as they are wide; without this the gradient would look
/// steeper than the angle asks for.
const CELL_ASPECT: f64 = 2.0;

/// Interpolates the evenly spaced stops in sRGB, like CSS does by default.
fn color_at(colors: &[[u8; 3]], mut t: f64) -> [u8; 3] {
    t *= (colors.len() - 1) as f64;
    let index = (t as usize).min(colors.len() - 2);
    let fraction = t - index as f64;
    let (start, end) = (colors[index], colors[index + 1]);
    [
        (start[0] as f64 + (end[0] as f64 - start[0] as f64) * fraction) as u8,
        (start[1] as f64 + (end[1] as f64 - start[1] as f64) * fraction) as u8,
        (start[2] as f64 + (end[2] as f64 - start[2] as f64) * fraction) as u8,
    ]
}

fn render(mut out: impl Write, lines: &[&str], stops: &[[u8; 3]]) {
    let width = lines.iter().map(|line| line.chars().count()).max().unwrap() as f64;
    let height = lines.len() as f64 * CELL_ASPECT;

    // CSS angles: 0deg points up, rotating clockwise, with y growing downwards.
    let radians = ANGLE.to_radians();
    let (dx, dy) = (radians.sin(), -radians.cos());
    // Length of the gradient line, chosen so the corners land exactly on the first and last stops.
    let length = (width * dx).abs() + (height * dy).abs();

    out.write_all(b"\n").unwrap();
    let mut buffer = Vec::<u8>::new();
    for (line, row) in lines.iter().zip(0..) {
        let mut current = None;
        for (char, column) in line.chars().zip(0..) {
            if char != ' ' {
                let x = column as f64 + 0.5 - width / 2.0;
                let y = (row as f64 + 0.5) * CELL_ASPECT - height / 2.0;
                let color @ [r, g, b] = color_at(stops, (x * dx + y * dy) / length + 0.5);
                if Some(color) != current {
                    write!(&mut buffer, "\x1b[38;2;{r};{g};{b}m").unwrap();
                    current = Some(color);
                }
            }
            write!(&mut buffer, "{char}").unwrap();
        }
        if current.is_some() {
            buffer.extend(b"\x1b[0m");
        }
        buffer.push(b'\n');
        out.write_all(&buffer).unwrap();
        buffer.clear();
    }
    out.write_all(b"\n").unwrap();
}

pub fn print() {
    let mode = std::io::stderr()
        .is_terminal()
        .then(|| terminal_colorsaurus::theme_mode(QueryOptions::default()).ok())
        .flatten()
        .unwrap_or(ThemeMode::Dark);

    let stops = match mode {
        ThemeMode::Dark => DARK_STOPS,
        ThemeMode::Light => LIGHT_STOPS,
    };

    let mut out = anstream::stderr().lock();
    if std::env::var("CHIPLAB_TEST_ALL_BANNERS").is_ok() {
        for banner in BANNERS {
            render(&mut out, banner, stops);
        }
    } else {
        type HashMap<K, V> = std::collections::HashMap<K, V, std::hash::RandomState>;
        // Simplest way to shuffle from std, `RandomState` makes the iteration order
        // non-deterministic so the first element is randomized each time.
        let banners = HashMap::from_iter((0..).zip(BANNERS));
        let (_, &banner) = banners.iter().next().unwrap();
        render(&mut out, banner, stops);
    }
}
