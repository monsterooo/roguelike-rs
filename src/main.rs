use rand::Rng;
use std::cmp;

use tcod::colors::*;
use tcod::console::*;
use tcod::input::Key;
use tcod::input::KeyCode::*;

// 窗口实际大小
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
// 最大每秒20帧
const LIMIT_FPS: i32 = 20;
// 地图大小
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;
// 地图颜色
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
// 地牢生成器
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

type Map = Vec<Vec<Tile>>;

// 与libtocd相关的值
struct Tcod {
    root: Root,
    con: Offscreen,
}

struct Game {
    map: Map,
}

/// 这是一个通用对象的抽：玩家、怪物、物品、楼梯等
/// 它始终由屏幕上的字符表示
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
}

/// 地图的瓦片和它的属性
#[derive(Clone, Copy, Debug)]
struct Tile {
    /// 该块是否被阻挡无法移动到此处
    blocked: bool,
    /// 阻挡视线，目前定义：墙(true) false(地面)
    block_sight: bool,
}

/// 一个在地图上的矩形，用于表示房间
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Object {
    /// 快捷方法创建一个对象
    pub fn new(x: i32, y: i32, char: char, color: Color) -> Self {
        Self { x, y, char, color }
    }

    /// 移动给定的值
    pub fn move_by(&mut self, dx: i32, dy: i32, game: &Game) {
        if !game.map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    /// 设置颜色然后再当前位置绘制对象的字符
    // &mut dyn Console 这里的限定表示 con 只要实现 Console trait 即可。
    // 这种限定方法叫 trait object
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
}

impl Tile {
    pub fn empty() -> Self {
        Self {
            blocked: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Self {
            blocked: true,
            block_sight: true,
        }
    }
}

impl Rect {
    /// 创建一个矩形
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    /// 获取中心点
    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    /// 如果一个图形与另一个图形相交返回 true
    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/rouguelike")
        .init();
    // 离屏渲染
    let con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);
    let mut tcod = Tcod { root, con };

    tcod::system::set_fps(LIMIT_FPS);

    let player = Object::new(0, 0, '@', WHITE);
    let npc = Object::new(SCREEN_WIDTH / 2 - 5, SCREEN_HEIGHT / 2, '@', YELLOW);
    let mut objects = [player, npc];
    let game = Game {
        map: make_map(&mut objects[0]),
    };

    // 主循环
    while !tcod.root.window_closed() {
        // 清除离屏的上一次渲染
        tcod.con.clear();
        render_all(&mut tcod, &game, &objects);
        tcod.root.flush();

        let player = &mut objects[0];
        let exit = handle_keys(&mut tcod, &game, player);

        if exit {
            break;
        }
    }
}

fn render_all(tcod: &mut Tcod, game: &Game, objects: &[Object]) {
    // 遍历所有瓦片并设置他们的背景颜色
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let wall = game.map[x as usize][y as usize].block_sight;
            if wall {
                tcod.con
                    .set_char_background(x, y, COLOR_DARK_WALL, BackgroundFlag::Set);
            } else {
                tcod.con
                    .set_char_background(x, y, COLOR_DARK_GROUND, BackgroundFlag::Set);
            }
        }
    }

    for object in objects {
        object.draw(&mut tcod.con);
    }

    // 拷贝后台渲染到前台
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

fn handle_keys(tcod: &mut Tcod, game: &Game, player: &mut Object) -> bool {
    let key = tcod.root.wait_for_keypress(true);
    match key {
        Key { code: Up, .. } => player.move_by(0, -1, game),
        Key { code: Down, .. } => player.move_by(0, 1, game),
        Key { code: Left, .. } => player.move_by(-1, 0, game),
        Key { code: Right, .. } => player.move_by(1, 0, game),
        Key {
            code: Enter,
            alt: true,
            ..
        } => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
        }
        Key { code: Escape, .. } => return true,
        _ => {}
    }
    false
}

fn make_map(player: &mut Object) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // 随机房间宽高
        let w = rand::thread_rng().gen_range((ROOM_MIN_SIZE..ROOM_MAX_SIZE + 1));
        let h = rand::thread_rng().gen_range((ROOM_MIN_SIZE..ROOM_MAX_SIZE + 1));
        // 随机房间位置，保证在地图内
        let x = rand::thread_rng().gen_range((0..MAP_WIDTH - w));
        let y = rand::thread_rng().gen_range((0..MAP_HEIGHT - h));

        let new_room = Rect::new(x, y, w, h);

        // 判断所有已存在的房间是否和新创建的房间相交
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            // 有效房间，绘制在地图上
            create_room(new_room, &mut map);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // 玩家从第一个房间开始
                player.x = new_x;
                player.y = new_y;
            } else {
                // 我们可以从一个水平隧道开始，到达与新房间相同的高度，然后与一个垂直隧道相连，或者我们可以做相反的事情:从一个垂直隧道开始，以一个水平隧道结束。

                // 前一个房间的中心点
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // 随机 true 和 false 对应两种不同的通道方式
                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }

            rooms.push(new_room);
        }
    }

    map
}

/// 将一个矩形放置在图上，并确保其地图快是空的
fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

// 创建水平隧道
fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    // `min()` 和 `max()` 用于 `x1 > x2` 的情况
    // 确保..能有正确的值返回
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

// 垂直隧道
fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}
