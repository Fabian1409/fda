use anyhow::Result;
use crossterm::{
    cursor::{self},
    event::{self, Event, KeyCode},
    style::{self, Stylize},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand, QueueableCommand,
};
use rand::{seq::IteratorRandom, Rng};
use std::{
    collections::HashMap,
    env,
    fs::read_to_string,
    io::{self, Write},
    time::{Duration, Instant},
};
use strum::EnumIter;
use strum::IntoEnumIterator;

#[derive(EnumIter, Clone)]
enum Direction {
    Up,
    UpLeft,
    UpRight,
    Down,
    DownLeft,
    DownRight,
    Left,
    Right,
}

impl Direction {
    fn offset(&self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::UpLeft => (-1, -1),
            Direction::UpRight => (1, -1),
            Direction::Down => (0, 1),
            Direction::DownLeft => (-1, 1),
            Direction::DownRight => (1, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

type State = Vec<usize>;
type Action = usize;

struct QLearn {
    q: HashMap<(State, Action), f32>,
    epsilon: f32,
    alpha: f32,
    gamma: f32,
}

impl QLearn {
    fn new(epsilon: f32, alpha: f32, gamma: f32) -> QLearn {
        QLearn {
            q: HashMap::new(),
            epsilon,
            alpha,
            gamma,
        }
    }

    fn get_q(&self, state: State, action: Action) -> f32 {
        *self.q.get(&(state, action)).unwrap_or(&0.0)
    }

    fn learn_q(&mut self, state: State, action: Action, reward: f32, value: f32) {
        if let Some(old_value) = self.q.get(&(state.clone(), action)) {
            self.q.insert(
                (state, action),
                old_value + self.alpha * (value - old_value),
            );
        } else {
            self.q.insert((state, action), reward);
        }
    }

    fn learn(&mut self, state1: State, action1: Action, reward: f32, state2: State) {
        let mut max_q = f32::MIN;
        for q in (0..7).filter_map(|action| self.q.get(&(state1.clone(), action))) {
            if *q > max_q {
                max_q = *q;
            }
        }
        self.learn_q(state1, action1, reward, reward + self.gamma * max_q);
    }

    fn choose_action(&self, state: State) -> Action {
        let mut rng = rand::thread_rng();
        if rand::random::<f32>() < self.epsilon {
            rng.gen_range(0..7)
        } else {
            let mut max_q = f32::MIN;
            let mut action = 0;
            for (a, q) in (0..7)
                .filter_map(|action| self.q.get(&(state.clone(), action)).map(|q| (action, q)))
            {
                if *q > max_q {
                    max_q = *q;
                    action = a;
                }
            }
            action
        }
    }
}

trait Agent {
    fn move_in_dir(
        &mut self,
        grid: &[Vec<Cell>],
        pos: (usize, usize),
        dir: Direction,
    ) -> Option<(usize, usize)> {
        let (dx, dy) = dir.offset();
        let x = (pos.0 as i32).saturating_add(dx) as usize % grid[0].len();
        let y = (pos.1 as i32).saturating_add(dy) as usize % grid.len();
        match grid[y][x] {
            Cell::Wall => None,
            _ => Some((x, y)),
        }
    }

    fn move_towards(
        &mut self,
        grid: &[Vec<Cell>],
        pos: (usize, usize),
        target: (usize, usize),
    ) -> Option<(usize, usize)> {
        if pos == target {
            return Some(pos);
        }

        let mut best = None;
        let mut best_dist = i32::MAX;

        for (dx, dy) in Direction::iter().map(|x| x.offset()) {
            let x = (pos.0 as i32).saturating_add(dx) as usize % grid[0].len();
            let y = (pos.1 as i32).saturating_add(dy) as usize % grid.len();

            if matches!(grid[y][x], Cell::Wall) {
                continue;
            }

            if target == (x, y) {
                best = Some((x, y));
                break;
            }

            let dist = (x as i32 - target.0 as i32).pow(2) + (y as i32 - target.1 as i32).pow(2);

            if best.is_none() || best_dist > dist {
                best = Some((x, y));
                best_dist = dist;
            }
        }
        best
    }
}

#[derive(Clone)]
enum Cell {
    Wall,
    Empty,
    Cheese,
    Cat,
    Mouse,
}

struct Cheese {
    pos: (usize, usize),
}

impl Cheese {
    fn new(x: usize, y: usize) -> Cheese {
        Cheese { pos: (x, y) }
    }
}

struct Cat {
    pos: (usize, usize),
    dir: Direction,
}

impl Cat {
    fn new(x: usize, y: usize, dir: Direction) -> Cat {
        Cat { pos: (x, y), dir }
    }
}

impl Agent for Cat {}

struct Mouse {
    pos: (usize, usize),
    dir: Direction,
    ai: QLearn,
}

impl Mouse {
    fn new(x: usize, y: usize, dir: Direction) -> Mouse {
        Mouse {
            pos: (x, y),
            dir,
            ai: QLearn::new(0.1, 0.1, 0.9),
        }
    }
}

impl Agent for Mouse {}

struct World {
    grid: Vec<Vec<Cell>>,
    cheese: Cheese,
    cat: Cat,
    mouse: Mouse,
    age: usize,
    eaten: usize,
    fed: usize,
    last_state: Option<State>,
    last_action: Option<Action>,
}

impl World {
    fn new(world_file: String, cheese: Cheese, cat: Cat, mouse: Mouse) -> World {
        let mut grid: Vec<Vec<Cell>> = read_to_string(world_file)
            .expect("failed to open")
            .lines()
            .map(|l| {
                l.chars()
                    .map(|c| match c {
                        'X' => Cell::Wall,
                        _ => Cell::Empty,
                    })
                    .collect()
            })
            .collect();
        grid[cheese.pos.1][cheese.pos.0] = Cell::Cheese;
        grid[cat.pos.1][cat.pos.0] = Cell::Cat;
        grid[mouse.pos.1][mouse.pos.0] = Cell::Mouse;
        World {
            grid,
            cheese,
            cat,
            mouse,
            age: 0,
            fed: 0,
            eaten: 0,
            last_state: None,
            last_action: None,
        }
    }

    fn update_cat(&mut self) {
        let old_pos = self.cat.pos;

        if old_pos != self.mouse.pos {
            let mut new_pos = self
                .cat
                .move_towards(&self.grid, old_pos, self.mouse.pos)
                .unwrap();
            let mut rng = rand::thread_rng();
            while new_pos == old_pos {
                new_pos = self
                    .cat
                    .move_in_dir(
                        &self.grid,
                        old_pos,
                        Direction::iter().choose(&mut rng).unwrap(),
                    )
                    .unwrap();
            }

            self.cat.pos = new_pos;
        }
    }

    fn calc_state(&self, pos: (usize, usize), d: i32) -> State {
        let mut state = Vec::new();
        for i in -d..d + 1 {
            for j in -d..d + 1 {
                let x = (pos.0 as i32).saturating_add(i) as usize % self.grid[0].len();
                let y = (pos.1 as i32).saturating_add(j) as usize % self.grid.len();

                let v = match self.grid[y][x] {
                    Cell::Wall => 1,
                    Cell::Cheese => 2,
                    Cell::Cat => 3,
                    _ => 0,
                };

                state.push(v);
            }
        }
        state
    }

    fn update_mouse(&mut self) {
        let state = self.calc_state(self.mouse.pos, 2);

        let mut reward: f32 = -1.0;

        if self.mouse.pos == self.cat.pos {
            self.eaten += 1;
            reward = -100.0;
            if let (Some(last_state), Some(last_action)) =
                (self.last_state.clone(), self.last_action)
            {
                self.mouse.ai.learn(last_state, last_action, reward, state);
            }
            self.last_state = None;
            self.last_action = None;
            self.mouse.pos = self.rand_pos();
            return;
        }

        if self.mouse.pos == self.cheese.pos {
            self.fed += 1;
            reward = 50.0;
            self.cheese.pos = self.rand_pos();
        }

        if let (Some(last_state), Some(last_action)) = (self.last_state.clone(), self.last_action) {
            self.mouse.ai.learn(last_state, last_action, reward, state);
        }

        let state = self.calc_state(self.mouse.pos, 2);
        let action = self.mouse.ai.choose_action(state.clone());

        self.last_state = Some(state);
        self.last_action = Some(action);

        let new_pos = self
            .mouse
            .move_in_dir(
                &self.grid,
                self.mouse.pos,
                Direction::iter().nth(action).unwrap(),
            )
            .unwrap_or(self.mouse.pos);
        self.mouse.pos = new_pos;
    }

    fn rand_pos(&self) -> (usize, usize) {
        let mut rng = rand::thread_rng();
        loop {
            let x = rng.gen_range(0..self.grid[0].len());
            let y = rng.gen_range(0..self.grid.len());

            if matches!(self.grid[y][x], Cell::Empty) {
                return (x, y);
            }
        }
    }

    fn run(&mut self) -> Result<()> {
        let mut stdout = io::stdout();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(1000);
        loop {
            stdout.execute(terminal::Clear(terminal::ClearType::All))?;
            for (y, row) in self.grid.iter().enumerate() {
                for (x, c) in row.iter().enumerate() {
                    stdout.queue(cursor::MoveTo(x as u16, y as u16))?;
                    match c {
                        Cell::Wall => write!(stdout, "X")?,
                        Cell::Cheese => {
                            stdout.queue(style::PrintStyledContent('#'.yellow()))?;
                        }
                        Cell::Cat => {
                            stdout.queue(style::PrintStyledContent('C'.red()))?;
                        }
                        Cell::Mouse => {
                            stdout.queue(style::PrintStyledContent('M'.grey()))?;
                        }
                        _ => {}
                    }
                }
            }
            stdout.queue(cursor::MoveToNextLine(1))?;
            stdout.write_fmt(format_args!("age = {}", self.age))?;
            stdout.queue(cursor::MoveToNextLine(1))?;
            stdout.write_fmt(format_args!("fed = {}", self.fed))?;
            stdout.queue(cursor::MoveToNextLine(1))?;
            stdout.write_fmt(format_args!("eaten = {}", self.eaten))?;

            stdout.flush()?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.grid[self.cheese.pos.1][self.cheese.pos.0] = Cell::Empty;
                self.grid[self.mouse.pos.1][self.mouse.pos.0] = Cell::Empty;
                self.grid[self.cat.pos.1][self.cat.pos.0] = Cell::Empty;
                self.update_cat();
                self.update_mouse();
                self.grid[self.cheese.pos.1][self.cheese.pos.0] = Cell::Cheese;
                self.grid[self.mouse.pos.1][self.mouse.pos.0] = Cell::Mouse;
                self.grid[self.cat.pos.1][self.cat.pos.0] = Cell::Cat;
                self.age += 1;
                last_tick = Instant::now();
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let world_file = env::args().nth(1).expect("filename required");
    let cheese = Cheese::new(1, 1);
    let cat = Cat::new(7, 6, Direction::Right);
    let mouse = Mouse::new(1, 2, Direction::Right);
    let mut world = World::new(world_file, cheese, cat, mouse);

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(cursor::Hide)?;

    world.run()?;

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    io::stdout().execute(cursor::Show)?;
    Ok(())
}
