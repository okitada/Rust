/*
rs2048.rs - 2048 Game

2017/01/07 pprof効果がわかるようにget_gapからlevel=0を別関数に分離(get_gap1)
2017/01/21 memprofile オプション追加 2048.exe -memprofile=6060 で実行し、http://localhost:6060/debug/pprof/heap?debug=1 を開く
2017/01/24 get_gapをチューニング（appear途中で最大値を超えたら枝刈りで読み中断）
2017/01/27 get_gap,get_gap1をチューニング（appear前のGapを1度計算しておいてから、各appearによる差分のみを加算）
2017/02/11 calc_gapをチューニング（端と端以外のGap計算時に端の方が小さければGapを増やす。-CALC_GAP_MODE追加）
2017/02/12 D_BONUS_POINT_MAX, D_BONUS2廃止
2019/10/05 calc_gapのバグ修正
2020/06/20 Go版からRust版へ移植開始
2020/06/27 Rust版一旦動作OK（残：乱数の種対応）

USAGE:
    rs2048.exe [OPTIONS]

FLAGS:
    -h, --help       Prints help information
        --version    Prints version information

OPTIONS:
    -a, --auto_mode <auto_mode>                                    読みの深さ(>0)
    -c, --calc_gap_mode <calc_gap_mode>
            gap計算モード(0:normal 1:端の方が小さければ+1 2:*2 3:+大きい方の値 4:+大きい方の値/10 5:+両方の値)

    -o, --one_time <one_time>                                      N回で終了
    -u, --pause_mode <pause_mode>                                  終了時に一時中断(0/1)
    -m, --print_mode <print_mode>                                  途中経過の表示間隔(0：表示しない)
    -t, --print_mode_turbo <print_mode_turbo>
            0:PRINT_MODEに従う 1:TURBO_MINUS_SCOREを超えたら強制表示 2:TURBO_PLUS_SCOREを超えたら強制表示

    -r, --seed <seed>                                              乱数の種
    -p, --turbo_minus_percent <turbo_minus_percent>                空き率がこれ以上であれば読みの深さを下げる
    -l, --turbo_minus_percent_level <turbo_minus_percent_level>    空き率が閾値以上の時に下げる読みの深さ
    -s, --turbo_minus_score <turbo_minus_score>                    点数がこれ以下であれば読みの深さを下げる
    -v, --turbo_minus_score_level <turbo_minus_score_level>        点数が閾値以下の時に下げる読みの深さ
    -P, --turbo_plus_percent <turbo_plus_percent>                  空き率がこれ以下であれば読みの深さを上げる
    -L, --turbo_plus_percent_level <turbo_plus_percent_level>      空き率が閾値以下の時に上げる読みの深さ
    -S, --turbo_plus_score <turbo_plus_score>                      点数がこれ以上であれば読みの深さを上げる
    -V, --turbo_plus_score_levels <turbo_plus_score_level>         点数が閾値以上の時に上げる読みの深さ

Game Over! (level=4 SEED=-184) 2020/06/28 01:32:15 #1 Ave.=121688 Max=121688(SEED=-184) Min=121688(SEED=-184)
getGap=349234643 calc_gap=6657041939 10.0,0.0 55%,1 20000,1 10%,1 200000,1 2 CALC_GAP_MODE=0
Rust[1:4540] 121688 (0.00/78.5 sec) 75000023.906250 2020/06/28 01:32:15 SEED=-184 2=75.17% Ave.=243376
 2048    16     2    16
   16   512   256    64
    8    32   128    32
    2     4  8192     8
Total time = 78.52 (sec)
    
Game Over! (level=4 SEED=-46) 2020/06/28 00:01:33 #1 Ave.=120736 Max=120736(SEED=-46) Min=120736(SEED=-46)
getGap=360710293 calc_gap=7405751820 10.0,0.0 55%,1 20000,1 10%,1 200000,1 2 CALC_GAP_MODE=0
Rust[1:4511] 120736 (0.00/84.6 sec) 75000001.108924 2020/06/28 00:01:33 SEED=-46 2=76.10% Ave.=241472
    2     8     2     4
    4    16    32    16
   64   128   512  2048
    8    16   128  8192
Total time = 84.61 (sec)


 .\target\release\rs2048.exe -r=-46 -S 111000 -t 2
 Game Over! (level=4 SEED=-46) 2020/06/28 00:26:14 #1 Ave.=143964 Max=143964(SEED=-46) Min=143964(SEED=-46)
getGap=4206140449 calc_gap=90626303617 10.0,0.0 55%,1 20000,1 10%,1 111000,1 2 CALC_GAP_MODE=0
Rust[1:5584] 143964 (0.00/1057.1 sec) 100000000.000000 2020/06/28 00:26:14 SEED=-46 2=78.18% Ave.=287928
    2     8    64     8
    8  2048   512    16
   32   128  2048  8192
    8   512    16     4
Total time = 1057.11 (sec)


*/

extern crate clap;
extern crate rand;
extern crate chrono;

use std::io;
use clap::{App,Arg};
use chrono::prelude::*;

static mut AUTO_MODE: i32 = 4; // >=0 depth;
static mut CALC_GAP_MODE: i32 = 0; // gap計算モード(0:normal 1:端の方が小さければ+1 2:*2 3:+大きい方の値 4:+大きい方の値/10 5:+両方の値);
static mut PRINT_MODE: i32 = 100; // 途中経過の表示間隔(0：表示しない);
static mut PRINT_MODE_TURBO: i32 = 1;
static mut PAUSE_MODE: i32 = 0;
static mut ONE_TIME: i32 = 1; // 繰り返し回数;
static mut SEED: i64 = 1;
static mut TURBO_MINUS_PERCENT      : i32 = 55;
static mut TURBO_MINUS_PERCENT_LEVEL: i32 = 1;
static mut TURBO_MINUS_SCORE        : i32 = 20000;
static mut TURBO_MINUS_SCORE_LEVEL  : i32 = 1;
static mut TURBO_PLUS_PERCENT       : i32 = 10;
static mut TURBO_PLUS_PERCENT_LEVEL : i32 = 1;
static mut TURBO_PLUS_SCORE         : i32 = 200000;
static mut TURBO_PLUS_SCORE_LEVEL   : i32 = 1;

const D_BONUS: f64 = 10.0;
const D_BONUS_USE_MAX: bool = true; //10固定ではなく最大値とする;
const GAP_EQUAL: f64 = 0.0;

const INIT2: i32 = 1;
const INIT4: i32 = 2;
const RNDMAX: i32 = 4;
const GAP_MAX: f64 = 100000000.0;
const XMAX: i32 = 4;
const YMAX: i32 = 4;
const XMAX_1: i32 = XMAX-1;
const YMAX_1: i32 = YMAX-1;
const RNDCYCLE: i32 = 1587;
const TICKS_PER_SEC: f64 = 1000000000.0;

static mut BOARD:[[i32; XMAX as usize]; YMAX as usize]=[[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]];
static mut SP: i32 = 0;

static mut POS_X:[i32;(XMAX*YMAX) as usize]=[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,];
static mut POS_Y:[i32;(XMAX*YMAX) as usize]=[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,];
static mut SCORE: i32 = 0;
static mut GEN: i32 = 0;
static mut COUNT_2: i32 = 0;
static mut COUNT_4: i32 = 0;
static mut COUNT_CALC_GAP: u64 = 0;
static mut COUNT_GET_GAP: u64 = 0;

static mut START_TIME: f64 = 0.0;
static mut LAST_TIME: f64 = 0.0;
static mut TOTAL_START_TIME: f64 = 0.0;
static mut TOTAL_LAST_TIME: f64 = 0.0;

static mut COUNT: i32 = 1;
static mut SUM_SCORE: i32 = 0;
static mut MAX_SCORE: i32 = 0;
static mut MAX_SEED: i64 = 0;
static mut MIN_SCORE: i32 = std::i32::MAX;
static mut MIN_SEED: i64 = 0;

//static mut rng = rand::thread_rng(); //デフォルトの乱数ジェネレータ
static mut RAND_NOW: i32 = 0;

fn main() {
    let app = App::new("rs2048")
        .version("0.1.0")
        .author("okitada")
        .about("2048 CLI auto game(Rust version)")
        .arg(Arg::with_name("auto_mode")
            .help("読みの深さ(>0)")
            .short("a")
            .long("auto_mode")
            .takes_value(true))
        .arg(Arg::with_name("calc_gap_mode")
            .help("gap計算モード(0:normal 1:端の方が小さければ+1 2:*2 3:+大きい方の値 4:+大きい方の値/10 5:+両方の値)")
            .short("c")
            .long("calc_gap_mode")
            .takes_value(true))
        .arg(Arg::with_name("print_mode")
            .help("途中経過の表示間隔(0：表示しない)")
            .short("m")
            .long("print_mode")
            .takes_value(true))
        .arg(Arg::with_name("print_mode_turbo")
            .help("0:PRINT_MODEに従う 1:TURBO_MINUS_SCOREを超えたら強制表示 2:TURBO_PLUS_SCOREを超えたら強制表示")
            .short("t")
            .long("print_mode_turbo")
            .takes_value(true))
        .arg(Arg::with_name("pause_mode")
            .help("終了時に一時中断(0/1)")
            .short("u")
            .long("pause_mode")
            .takes_value(true))
        .arg(Arg::with_name("seed")
            .help("乱数の種")
            .short("r")
            .long("seed")
            .takes_value(true))
        .arg(Arg::with_name("one_time")
            .help("N回で終了")
            .short("o")
            .long("one_time")
            .takes_value(true))
        .arg(Arg::with_name("turbo_minus_percent")
            .help("空き率がこれ以上であれば読みの深さを下げる")
            .short("p")
            .long("turbo_minus_percent")
            .takes_value(true))
        .arg(Arg::with_name("turbo_minus_percent_level")
            .help("空き率が閾値以上の時に下げる読みの深さ")
            .short("l")
            .long("turbo_minus_percent_level")
            .takes_value(true))
        .arg(Arg::with_name("turbo_minus_score")
            .help("点数がこれ以下であれば読みの深さを下げる")
            .short("s")
            .long("turbo_minus_score")
            .takes_value(true))
        .arg(Arg::with_name("turbo_minus_score_level")
            .help("点数が閾値以下の時に下げる読みの深さ")
            .short("v")
            .long("turbo_minus_score_level")
            .takes_value(true))
        .arg(Arg::with_name("turbo_plus_percent")
            .help("空き率がこれ以下であれば読みの深さを上げる")
            .short("P")
            .long("turbo_plus_percent")
            .takes_value(true))
        .arg(Arg::with_name("turbo_plus_percent_level")
            .help("空き率が閾値以下の時に上げる読みの深さ")
            .short("L")
            .long("turbo_plus_percent_level")
            .takes_value(true))
        .arg(Arg::with_name("turbo_plus_score")
            .help("点数がこれ以上であれば読みの深さを上げる")
            .short("S")
            .long("turbo_plus_score")
            .takes_value(true))
        .arg(Arg::with_name("turbo_plus_score_level")
            .help("点数が閾値以上の時に上げる読みの深さ")
            .short("V")
            .long("turbo_plus_score_levels")
            .takes_value(true));
    let matches = app.get_matches();

    unsafe {
        
    if let Some(o) = matches.value_of("auto_mode") {
        AUTO_MODE = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("calc_gap_mode") {
        CALC_GAP_MODE = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("print_mode") {
        PRINT_MODE = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("print_mode_turbo") {
        PRINT_MODE_TURBO = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("pause_mode") {
        PAUSE_MODE = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("seed") {
        SEED = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("one_time") {
        ONE_TIME = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_minus_percent") {
        TURBO_MINUS_PERCENT = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_minus_percent_level") {
        TURBO_MINUS_PERCENT_LEVEL = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_minus_score") {
        TURBO_MINUS_SCORE = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_minus_score_level") {
        TURBO_MINUS_SCORE_LEVEL = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_plus_percent") {
        TURBO_PLUS_PERCENT = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_plus_percent_level") {
        TURBO_PLUS_PERCENT_LEVEL = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_plus_score") {
        TURBO_PLUS_SCORE = o.parse().unwrap();
    }
    if let Some(o) = matches.value_of("turbo_plus_score_level") {
        TURBO_PLUS_SCORE_LEVEL = o.parse().unwrap();
    }

    println!("AUTO_MODE={}", AUTO_MODE);
    println!("CALC_GAP_MODE={}", CALC_GAP_MODE);
    println!("PRINT_MODE={}", PRINT_MODE);
    println!("PRINT_MODE_TURBO={}", PRINT_MODE_TURBO);
    println!("PAUSE_MODE={}", PAUSE_MODE);
    println!("SEED={}", SEED);
    println!("ONE_TIME={}", ONE_TIME);
    println!("TURBO_MINUS_PERCENT={}", TURBO_MINUS_PERCENT);
    println!("TURBO_MINUS_PERCENT_LEVEL={}", TURBO_MINUS_PERCENT_LEVEL);
    println!("TURBO_MINUS_SCORE={}", TURBO_MINUS_SCORE);
    println!("TURBO_MINUS_SCORE_LEVEL={}", TURBO_MINUS_SCORE_LEVEL);
    println!("TURBO_PLUS_PERCENT={}", TURBO_PLUS_PERCENT);
    println!("TURBO_PLUS_PERCENT_LEVEL={}", TURBO_PLUS_PERCENT_LEVEL);
    println!("TURBO_PLUS_SCORE={}", TURBO_PLUS_SCORE);
    println!("TURBO_PLUS_SCORE_LEVEL={}", TURBO_PLUS_SCORE_LEVEL);

    if SEED > 0 {
        //SeedableRng::from_rng(SEED);
    } else if SEED < 0 {
        RAND_NOW = -SEED as i32 -1;
    } else {
        //StdRng::from_entropy();
    }
    TOTAL_START_TIME = get_now();
    init_game();
    loop {
        let gap: f64 = move_auto(AUTO_MODE);
        GEN+=1;
        appear();
        disp(gap, PRINT_MODE > 0 &&
            (GEN%PRINT_MODE==0 ||
                (PRINT_MODE_TURBO==1 && SCORE>TURBO_MINUS_SCORE) ||
                (PRINT_MODE_TURBO==2 && SCORE>TURBO_PLUS_SCORE)));
        if is_gameover() {
            let sc:i32 = get_score();
            SUM_SCORE += sc;
            if sc > MAX_SCORE {
                MAX_SCORE = sc;
                MAX_SEED = SEED;
            }
            if sc < MIN_SCORE {
                MIN_SCORE = sc;
                MIN_SEED = SEED;
            }
            print!("Game Over! (level={} SEED={}) {} #{} Ave.={} Max={}(SEED={}) Min={}(SEED={})\ngetGap={} calc_gap={} {:.1},{:.1} {}%,{} {},{} {}%,{} {},{} {} CALC_GAP_MODE={}\n",
                AUTO_MODE, SEED,
                get_time_str(), COUNT, SUM_SCORE/COUNT,
                MAX_SCORE, MAX_SEED, MIN_SCORE, MIN_SEED,
                COUNT_GET_GAP, COUNT_CALC_GAP,
                D_BONUS, GAP_EQUAL,
                TURBO_MINUS_PERCENT, TURBO_MINUS_PERCENT_LEVEL,
                TURBO_MINUS_SCORE, TURBO_MINUS_SCORE_LEVEL,
                TURBO_PLUS_PERCENT, TURBO_PLUS_PERCENT_LEVEL,
                TURBO_PLUS_SCORE, TURBO_PLUS_SCORE_LEVEL,
                PRINT_MODE_TURBO, CALC_GAP_MODE);
            disp(gap, true);
            if ONE_TIME > 0 {
                ONE_TIME-=1;
                if ONE_TIME == 0 {
                    break;
                }
            }
            if PAUSE_MODE > 0 {
                let mut key = String::new();
                match io::stdin().read_line(&mut key) {
                    Ok(_n)=> {
                        if key == "q" {
                            break;
                        }
                    }
                    Err(error)=> println!("error: {}", error),
                }
            }
            SEED+=1;
            if SEED > 0 {
                //SeedableRng::from_rng(SEED);
            } else if SEED < 0 {
                RAND_NOW = -SEED as i32 -1;
            } else {
                //StdRng::from_entropy();
            }
            init_game();
            COUNT+=1;
        }
    }
    TOTAL_LAST_TIME = get_now();
    println!("Total time = {:.2} (sec)", (TOTAL_LAST_TIME-TOTAL_START_TIME)/TICKS_PER_SEC as f64);

    }//unsafe
}

unsafe fn get_now()-> f64 {
    let local: DateTime<Local> = Local::now();
    let secs = local.timestamp() as f64;
    let nanos = local.timestamp_subsec_nanos() as f64;
    secs *1e9 + nanos
}

unsafe fn get_time_str()-> String {
    let local: DateTime<Local> = Local::now();
    //return format!("{}", local.to_string());
    //return format!("{:04}/{:02}/{:02} {:02}:{:02}:{02}", local.year(), local.month(), local.day(), local.hour(), local.minute(), local.second());
    return local.format("%Y/%m/%d %T").to_string(); //同 "%Y/%m/%d %H:%M:%S"
}

unsafe fn get_cell(x: i32, y: i32)-> i32 {
    return BOARD[x as usize][y as usize] as i32;
}

unsafe fn set_cell(x: i32, y: i32, n: i32)-> i32 {
    BOARD[x as usize][y as usize] = n;
    return n;
}

unsafe fn clear_cell(x: i32, y: i32) {
    set_cell(x, y, 0);
}

unsafe fn copy_cell(x1: i32, y1: i32, x2: i32, y2: i32)-> i32 {
    return set_cell(x2, y2, get_cell(x1, y1));
}

unsafe fn move_cell(x1: i32, y1: i32, x2: i32, y2: i32) {
    copy_cell(x1, y1, x2, y2);
    clear_cell(x1, y1);
}

unsafe fn add_cell(x1: i32, y1: i32, x2: i32, y2: i32) {
    BOARD[x2 as usize][y2 as usize]+=1;
    clear_cell(x1, y1);
    if SP < 1 {
        add_score(1 << (get_cell(x2, y2)));
    }
}

unsafe fn is_empty(x: i32, y: i32)-> bool {
    return get_cell(x, y) == 0;
}

unsafe fn is_not_empty(x: i32, y: i32)-> bool {
    return !is_empty(x, y);
}

unsafe fn is_gameover()-> bool {
    if {let (ret, _, _) = is_movable(); ret} {
        return false;
    } else {
        return true;
    }
}

unsafe fn get_score()-> i32 {
    return SCORE;
}

unsafe fn set_score(sc: i32)-> i32 {
    SCORE = sc;
    return SCORE;
}

unsafe fn add_score(sc: i32)-> i32 {
    SCORE += sc;
    return SCORE;
}

unsafe fn clear() {
    for y in 0..YMAX {
        for x in 0..XMAX {
            clear_cell(x, y);
        }
    }
}

unsafe fn disp(gap: f64, debug: bool) {
    let now = get_now();
    if COUNT == 0 {
        print!("Rust[{}:{}] {} ({:.2}/{:.1} sec) {:.6} {} SEED={} 2={:.2}%\r", COUNT, GEN, get_score(),(now-LAST_TIME as f64)/TICKS_PER_SEC,(now-START_TIME as f64)/TICKS_PER_SEC, gap, get_time_str(), SEED, (COUNT_2 as f64)/(COUNT_2+COUNT_4) as f64*100.0);
    } else {
        print!("Rust[{}:{}] {} ({:.2}/{:.1} sec) {:.6} {} SEED={} 2={:.2}% Ave.={}\r", COUNT, GEN, get_score(),(now-LAST_TIME as f64)/TICKS_PER_SEC,(now-START_TIME as f64)/TICKS_PER_SEC, gap, get_time_str(), SEED, (COUNT_2 as f64)/(COUNT_2+COUNT_4) as f64*100.0, (SUM_SCORE+get_score())/COUNT);
    }
    LAST_TIME = now;
    if debug {
        print!("\n");
        for y in 0..YMAX {
            for x in 0..XMAX {
                let v = get_cell(x, y);
                if v > 0 {
                    print!("{:5} ", 1<<v);
                } else {
                    print!("{:>5} ", ".");
                }
            }
            print!("\n");
        }
    }
}

unsafe fn init_game() {
    GEN = 1;
    set_score(0);
    START_TIME = get_now();
    LAST_TIME = START_TIME;
    clear();
    appear();
    appear();
    COUNT_2 = 0;
    COUNT_4 = 0;
    COUNT_CALC_GAP = 0;
    COUNT_GET_GAP = 0;
    disp(0.0, PRINT_MODE == 1);
}

unsafe fn get_random(n: i32)-> i32 {
    RAND_NOW=(RAND_NOW+1)%RNDCYCLE; //CRC-10
//println!("{}", RAND_NOW);
    RAND_NOW % n
}

unsafe fn appear()-> bool {
    let mut n: usize = 0;
    for y in 0..YMAX {
        for x in 0..XMAX {
            if is_empty(x, y) {
                POS_X[n] = x;
                POS_Y[n] = y;
                n+=1;
            }
        }
    }
    if n> 0 {
        let v: i32;
        let i: usize = get_random(n as i32) as usize;
        if get_random(RNDMAX) >= 1 {
            v = INIT2;
            COUNT_2+=1;
        } else {
            v = INIT4;
            COUNT_4+=1;
        }
        let x = POS_X[i];
        let y = POS_Y[i];
        set_cell(x, y, v);
        return true;
    }
    return false;
}

unsafe fn count_empty()-> i32 {
    let mut ret: i32 = 0;
    for y in 0..YMAX {
        for x in 0..XMAX {
            if is_empty(x, y) {
                ret+=1;
            }
        }
    }
    return ret;
}

unsafe fn move_up()-> i32 {
    let mut move_count: i32 = 0;
    let mut y_limit: i32;
    let mut y_next: i32;
    for x in 0..XMAX {
        y_limit = 0;
        for y in 1..YMAX {
            if is_not_empty(x, y) {
                y_next = y - 1;
                while y_next >= y_limit {
                    if is_not_empty(x, y_next) {
                        break;
                    }
                    if y_next == 0 {
                        break;
                    }
                    y_next = y_next - 1;
                }
                if y_next < y_limit {
                    y_next = y_limit;
                }
                if is_empty(x, y_next) {
                    move_cell(x, y, x, y_next);
                    move_count+=1;
                } else {
                    if get_cell(x, y_next) == get_cell(x, y) {
                        add_cell(x, y, x, y_next);
                        move_count+=1;
                        y_limit = y_next + 1;
                    } else {
                        if y_next+1 != y {
                            move_cell(x, y, x, y_next+1);
                            move_count+=1;
                            y_limit = y_next + 1;
                        }
                    }
                }
            }
        }
    }
    return move_count;
}

unsafe fn move_left()-> i32 {
    let mut move_count: i32 = 0;
    let mut x_limit: i32;
    let mut x_next: i32;
    for y in 0..YMAX {
        x_limit = 0;
        for x in 1..XMAX {
            if is_not_empty(x, y) {
                x_next = x - 1;
                while x_next >= x_limit {
                    if is_not_empty(x_next, y) {
                        break;
                    }
                    if x_next == 0 {
                        break;
                    }
                    x_next = x_next - 1;
                }
                if x_next < x_limit {
                    x_next = x_limit;
                }
                if is_empty(x_next, y) {
                    move_cell(x, y, x_next, y);
                    move_count+=1;
                } else {
                    if get_cell(x_next, y) == get_cell(x, y) {
                        add_cell(x, y, x_next, y);
                        move_count+=1;
                        x_limit = x_next + 1;
                    } else {
                        if x_next+1 != x {
                            move_cell(x, y, x_next+1, y);
                            move_count+=1;
                            x_limit = x_next + 1;
                        }
                    }
                }
            }
        }
    }
    return move_count;
}

unsafe fn move_down()-> i32 {
    let mut move_count: i32 = 0;
    let mut y_limit: i32;
    let mut y_next: i32;
    for x in 0..XMAX {
        y_limit = YMAX_1;
        for y in (0..YMAX_1).rev() {
            if is_not_empty(x, y) {
                y_next = y + 1;
                while y_next <= y_limit {
                    if is_not_empty(x, y_next) {
                        break;
                    }
                    if y_next == YMAX_1 {
                        break;
                    }
                    y_next = y_next + 1;
                }
                if y_next > y_limit {
                    y_next = y_limit;
                }
                if is_empty(x, y_next) {
                    move_cell(x, y, x, y_next);
                    move_count+=1;
                } else {
                    if get_cell(x, y_next) == get_cell(x, y) {
                        add_cell(x, y, x, y_next);
                        move_count+=1;
                        y_limit = y_next - 1;
                    } else {
                        if y_next-1 != y {
                            move_cell(x, y, x, y_next-1);
                            move_count+=1;
                            y_limit = y_next - 1;
                        }
                    }
                }
            }
        }
    }
    return move_count;
}

unsafe fn move_right()-> i32 {
    let mut move_count: i32 = 0;
    let mut x_limit: i32;
    let mut x_next: i32;
    for y in 0..YMAX {
        x_limit = XMAX_1;
        for x in (0..XMAX_1).rev() {
            if is_not_empty(x, y) {
                x_next = x + 1;
                while x_next <= x_limit {
                    if is_not_empty(x_next, y) {
                        break;
                    }
                    if x_next == XMAX_1 {
                        break;
                    }
                    x_next = x_next + 1;
                }
                if x_next > x_limit {
                    x_next = x_limit;
                }
                if is_empty(x_next, y) {
                    move_cell(x, y, x_next, y);
                    move_count+=1;
                } else {
                    if get_cell(x_next, y) == get_cell(x, y) {
                        add_cell(x, y, x_next, y);
                        move_count+=1;
                        x_limit = x_next - 1;
                    } else {
                        if x_next-1 != x {
                            move_cell(x, y, x_next-1, y);
                            move_count+=1;
                            x_limit = x_next - 1;
                        }
                    }
                }
            }
        }
    }
    return move_count;
}

unsafe fn move_auto(mut n_auto_mode: i32)-> f64 {
    let empty: i32 = count_empty();
    let sc = get_score();
    if empty >= XMAX*YMAX*TURBO_MINUS_PERCENT/100 {
        n_auto_mode -= TURBO_MINUS_PERCENT_LEVEL;
    } else if empty < XMAX*YMAX*TURBO_PLUS_PERCENT/100 {
        n_auto_mode += TURBO_PLUS_PERCENT_LEVEL;
    }
    if sc < TURBO_MINUS_SCORE {
        n_auto_mode -=TURBO_MINUS_SCORE_LEVEL;
    } else if sc >= TURBO_PLUS_SCORE {
        n_auto_mode += TURBO_PLUS_SCORE_LEVEL;
    }
    return move_best(n_auto_mode, true);
}

unsafe fn move_best(n_auto_mode: i32, move_count: bool)-> f64 {
    let mut n_gap: f64;
    let mut n_gap_best: f64;
    let mut n_dir_best: i32 = 0;
    let mut n_dir: i32 = 0;
    let board_bak = BOARD;
    SP+=1;
    n_gap_best = GAP_MAX;
    if move_up() > 0 {
        n_dir = 1;
        n_gap = get_gap(n_auto_mode, n_gap_best);
//println!("[{}] up    {}",n_auto_mode,n_gap);
        if n_gap < n_gap_best {
            n_gap_best = n_gap;
            n_dir_best = 1;
        }
    }
    BOARD = board_bak;
    if move_left() > 0 {
        n_dir = 2;
        n_gap = get_gap(n_auto_mode, n_gap_best);
//println!("[{}] left  {}",n_auto_mode,n_gap);
        if n_gap < n_gap_best {
            n_gap_best = n_gap;
            n_dir_best = 2;
        }
    }
    BOARD = board_bak;
    if move_down() > 0 {
        n_dir = 3;
        n_gap = get_gap(n_auto_mode, n_gap_best);
//println!("[{}] down  {}",n_auto_mode,n_gap);
        if n_gap < n_gap_best {
            n_gap_best = n_gap;
            n_dir_best = 3;
        }
    }
    BOARD = board_bak;
    if move_right() > 0 {
        n_dir = 4;
        n_gap = get_gap(n_auto_mode, n_gap_best);
//println!("[{}] right {}",n_auto_mode,n_gap);
        if n_gap < n_gap_best {
            n_gap_best = n_gap;
            n_dir_best = 4;
        }
    }
    BOARD = board_bak;
    SP-=1;
    if move_count {
        if n_dir_best == 0 {
            print!("\n***** Give UP *****\n");
            n_dir_best = n_dir;
        }
//println!("[{}] BEST  {}",n_auto_mode,n_dir_best);
        match n_dir_best {
            1 => move_up(),
            2 => move_left(),
            3 => move_down(),
            4 => move_right(),
            _ => 0,
        }; //何故かif の最後は文でないとエラーになる為に「;」を付けている
    }
    return n_gap_best;
}

unsafe fn get_gap(n_auto_mode: i32, n_gap_best: f64)-> f64 {
    COUNT_GET_GAP+=1;
    let mut ret: f64 = 0.0;
    let (movable, n_empty, n_bonus) = is_movable();
    if ! movable {
        ret = GAP_MAX;
    } else if n_auto_mode <= 1 {
        ret = get_gap1(n_gap_best, n_empty, n_bonus);
    } else {
        let alpha = n_gap_best * (n_empty as f64); //累積がこれを超えれば、平均してもn_gap_bestを超えるので即枝刈りする;
        for x in 0..XMAX {
            for y in 0..YMAX {
                if is_empty(x, y) {
                    set_cell(x, y, INIT2);
                    ret += move_best(n_auto_mode-1, false) * ((RNDMAX as f64 - 1.0) / RNDMAX as f64);
                    if ret >= alpha {
                        return GAP_MAX;    //枝刈り;
                    }
                    set_cell(x, y, INIT4);
                    ret += move_best(n_auto_mode-1, false) / RNDMAX as f64;
                    if ret >= alpha {
                        return GAP_MAX;    //枝刈り;
                    }
                    clear_cell(x, y);
                }
            }
        }
        ret /= n_empty as f64; //平均値を返す;
    }
    return ret;
}

unsafe fn get_gap1(n_gap_best: f64, n_empty: i32, n_bonus: f64)-> f64 {
    let mut ret: f64 = 0.0;
    let mut ret_appear: f64 = 0.0;
    let alpha: f64 = n_gap_best * n_bonus;
    let mut edgea: bool;
    let mut edgeb: bool;
    for x in 0..XMAX {
        for y in 0..YMAX {
            let v: i32 = get_cell(x, y);
            edgea = (x == 0 || y == 0) || (x == XMAX - 1 || y == YMAX_1);
            if v > 0 {
                if x < XMAX_1 {
                    let x1: i32 = get_cell(x+1, y);
                    edgeb = (y == 0) || (x+1 == XMAX - 1 || y == YMAX_1);
                    if x1 > 0 {
                        ret += calc_gap(v, x1, edgea, edgeb);
                    } else {
                        ret_appear += calc_gap(v, INIT2, edgea, edgeb) * ((RNDMAX as f64 - 1.0) / RNDMAX as f64);
                        ret_appear += calc_gap(v, INIT4, edgea, edgeb) / RNDMAX as f64;
                    }
                }
                if y < YMAX_1 {
                    let y1: i32 = get_cell(x, y+1);
                    edgeb = (x == 0) || (x == XMAX - 1 || y+1 == YMAX_1);
                    if y1 > 0 {
                        ret += calc_gap(v, y1, edgea, edgeb);
                    } else {
                        ret_appear += calc_gap(v, INIT2, edgea, edgeb) * ((RNDMAX as f64 - 1.0) / RNDMAX as f64);
                        ret_appear += calc_gap(v, INIT4, edgea, edgeb) / RNDMAX as f64;
                    }
                }
            } else {
                if x < XMAX_1 {
                    let x1: i32 = get_cell(x+1, y);
                    edgeb = (y == 0) || (x+1 == XMAX - 1 || y == YMAX_1);
                    if x1 > 0  {
                        ret_appear += calc_gap(INIT2, x1, edgea, edgeb) * ((RNDMAX as f64 - 1.0) / RNDMAX as f64);
                        ret_appear += calc_gap(INIT4, x1, edgea, edgeb) / RNDMAX as f64;
                    }
                }
                if y < YMAX_1 {
                    let y1: i32 = get_cell(x, y+1);
                    edgeb = (x == 0) || (x == XMAX - 1 || y+1 == YMAX_1);
                    if y1 > 0 {
                        ret_appear += calc_gap(INIT2, y1, edgea, edgeb) * ((RNDMAX as f64 - 1.0) / RNDMAX as f64);
                        ret_appear += calc_gap(INIT4, y1, edgea, edgeb) / RNDMAX as f64;
                    }
                }
            }
            if ret + ret_appear/(n_empty as f64) > alpha {
                return GAP_MAX;
            }
        }
    }
    ret += ret_appear / (n_empty as f64);
    ret /= n_bonus;
    return ret;
}

unsafe fn calc_gap(a: i32, b: i32, edgea: bool, edgeb: bool)-> f64 {
    COUNT_CALC_GAP+=1;
    let mut ret: f64;
    if a > b {
        ret = (a - b) as f64;
        if CALC_GAP_MODE > 0 && ! edgea && edgeb {
            match CALC_GAP_MODE {
            1 => ret += 1.0,
            2 => ret *= 2.0,
            3 => ret += a as f64,
            4 => ret += (a as f64)/10.0,
            5 => ret += (a+b) as f64,
            _ => (),
            }
        }
    } else if a < b {
        ret = (b - a) as f64;
        if CALC_GAP_MODE > 0 && edgea && ! edgeb {
            match CALC_GAP_MODE {
            1 => ret += 1.0,
            2 => ret *= 2.0,
            3 => ret += b as f64,
            4 => ret += (b as f64)/10.0,
            5 => ret += (a+b) as f64,
            _ => (),
            }
        }
    } else {
        ret = GAP_EQUAL;
    }
    return ret;
}

unsafe fn is_movable()-> (bool, i32, f64) {
    let mut ret: bool = false; //動けるか？;
    let mut n_empty: i32 = 0; //空きの数;
    let mut n_bonus: f64 = 1.0; //ボーナス（隅が最大値ならD_BONUS）;
    let mut max_x: i32 = -1;
    let mut max_y: i32 = -1;
    let mut max: i32 = 0;
    for y in 0..YMAX {
        for x in 0..XMAX {
            let val: i32 = get_cell(x, y);
            if val == 0 {
                ret = true;
                n_empty+=1;
            } else {
                if val > max {
                    max = val;
                    max_x = x;
                    max_y = y;
                }
                if ! ret {
                    if x < XMAX_1 {
                        let x1: i32 = get_cell(x+1, y);
                        if val == x1 || x1 == 0 {
                            ret = true;
                        }
                    }
                    if y < YMAX_1 {
                        let y1: i32 = get_cell(x, y+1);
                        if val == y1 || y1 == 0 {
                            ret = true;
                        }
                    }
                }
            }
        }
    }
    if (max_x == 0 || max_x == XMAX_1) &&
        (max_y == 0 || max_y == YMAX_1) {
        if D_BONUS_USE_MAX {
            n_bonus = max as f64;
        } else {
            n_bonus = D_BONUS;
        }
    }
    return (ret, n_empty, n_bonus);
}
