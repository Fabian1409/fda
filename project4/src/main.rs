use anyhow::Result;
use clap::{arg, command, value_parser};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{self, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use dijkstra::Dijkstra;
use plotters::prelude::*;
use rand::{seq::IteratorRandom, Rng};
use std::{
    collections::{HashMap, VecDeque},
    env,
    fs::read_to_string,
    io::{self, Write},
    time::{Duration, Instant},
};
use strum::EnumIter;
use strum::IntoEnumIterator;

mod dijkstra;

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
    q: HashMap<(State, Action), f64>,
    epsilon: f64,
    alpha: f64,
    gamma: f64,
}

impl QLearn {
    fn new(epsilon: f64, alpha: f64, gamma: f64) -> QLearn {
        QLearn {
            q: HashMap::new(),
            epsilon,
            alpha,
            gamma,
        }
    }

    fn get_q(&self, state: State, action: Action) -> f64 {
        *self.q.get(&(state, action)).unwrap_or(&0.0)
    }

    fn learn_q(&mut self, state: State, action: Action, reward: f64, value: f64) {
        if let Some(old_value) = self.q.get(&(state.clone(), action)) {
            self.q.insert(
                (state, action),
                old_value + self.alpha * (value - old_value),
            );
        } else {
            self.q.insert((state, action), reward);
        }
    }

    fn learn(&mut self, state1: State, action1: Action, reward: f64, state2: State) {
        let mut max_q = f64::MIN;
        for q in (0..7).map(|action| self.get_q(state2.clone(), action)) {
            if q > max_q {
                max_q = q;
            }
        }
        self.learn_q(state1, action1, reward, reward + self.gamma * max_q);
    }

    fn choose_action(&self, state: State) -> Action {
        let mut rng = rand::thread_rng();
        if rand::random::<f64>() < self.epsilon {
            rng.gen_range(0..7)
        } else {
            let mut max_q = f64::MIN;
            let mut chosen = 0;
            for (a, q) in (0..7).map(|action| (action, self.get_q(state.clone(), action))) {
                if q > max_q {
                    max_q = q;
                    chosen = a;
                }
            }
            chosen
        }
    }
}

type Position = (usize, usize);

trait Agent {
    fn move_in_dir(
        &mut self,
        grid: &[Vec<Cell>],
        pos: Position,
        dir: Direction,
    ) -> Option<Position> {
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
        pos: Position,
        target: Position,
    ) -> Option<Position> {
        if pos == target {
            return Some(pos);
        }

        let mut best = None;
        let mut best_dist = usize::MAX;

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

            let dist = ((x as i32 - target.0 as i32).pow(2) + (y as i32 - target.1 as i32).pow(2))
                as usize;

            if best.is_none() || best_dist > dist {
                best = Some((x, y));
                best_dist = dist;
            }
        }
        best
    }
}

#[derive(Clone, Debug)]
enum Cell {
    Wall,
    Empty,
    Cheese,
    Cat,
    Mouse,
}

struct Cheese {
    pos: Position,
}

impl Cheese {
    fn new(pos: Position) -> Cheese {
        Cheese { pos }
    }
}

struct Cat {
    pos: Position,
}

impl Cat {
    fn new(pos: Position) -> Cat {
        Cat { pos }
    }
}

impl Agent for Cat {}

struct Mouse {
    pos: Position,
    dumb: bool,
    ai: QLearn,
    eaten: usize,
    fed: usize,
    last_state: Option<State>,
    last_action: Option<Action>,
    last_cells: VecDeque<Position>,
    last_cheese_dist: usize,
    fed_stats: Vec<usize>,
    time_to_cheese_stats: Vec<usize>,
}

impl Mouse {
    fn new(pos: Position, dumb: bool) -> Mouse {
        Mouse {
            pos,
            dumb,
            ai: QLearn::new(0.1, 0.1, 0.9),
            eaten: 0,
            fed: 0,
            last_state: None,
            last_action: None,
            last_cells: VecDeque::new(),
            last_cheese_dist: usize::MAX,
            fed_stats: Vec::new(),
            time_to_cheese_stats: vec![0],
        }
    }
}

impl Agent for Mouse {}

fn rand_pos(grid: &[Vec<Cell>]) -> Position {
    let mut rng = rand::thread_rng();
    loop {
        let x = rng.gen_range(0..grid[0].len());
        let y = rng.gen_range(0..grid.len());

        if matches!(grid[y][x], Cell::Empty) {
            return (x, y);
        }
    }
}

struct World {
    grid: Vec<Vec<Cell>>,
    neighbors: HashMap<Position, Vec<Position>>,
    dijkstra: Dijkstra,
    epochs: usize,
    cheese: Cheese,
    cat: Cat,
    mouse: Mouse,
    age: usize,
    tick_rate: Duration,
    skip: usize,
    cat_enabled: bool,
    look_dist: usize,
    display: bool,
}

impl World {
    #[allow(clippy::too_many_arguments)]
    fn new(
        world_file: &str,
        epochs: usize,
        dumb_mouse: bool,
        skip: usize,
        tick_rate: Duration,
        cat_enabled: bool,
        display: bool,
    ) -> World {
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
        let mut neighbors: HashMap<Position, Vec<Position>> = HashMap::new();
        for (i, row) in grid.iter().enumerate() {
            for (j, c) in row.iter().enumerate() {
                let mut ns = Vec::new();
                if matches!(c, Cell::Empty) {
                    for dir in Direction::iter() {
                        let nx = j as i32 + dir.offset().0;
                        let ny = i as i32 + dir.offset().1;
                        if nx < 0 || nx > grid[0].len() as i32 - 1 {
                            continue;
                        }
                        if ny < 0 || ny > grid.len() as i32 - 1 {
                            continue;
                        }
                        if matches!(grid[ny as usize][nx as usize], Cell::Empty) {
                            ns.push((nx as usize, ny as usize));
                        }
                    }
                }
                neighbors.insert((j, i), ns);
            }
        }

        let cheese = Cheese::new(rand_pos(&grid));
        let cat = Cat::new(rand_pos(&grid));
        let mouse = Mouse::new(rand_pos(&grid), dumb_mouse);

        grid[cheese.pos.1][cheese.pos.0] = Cell::Cheese;
        grid[mouse.pos.1][mouse.pos.0] = Cell::Mouse;
        if cat_enabled {
            grid[cat.pos.1][cat.pos.0] = Cell::Cat;
        }
        World {
            grid,
            neighbors,
            dijkstra: Dijkstra::new(),
            epochs,
            cheese,
            cat,
            mouse,
            age: 0,
            tick_rate,
            skip,
            cat_enabled,
            look_dist: 2,
            display,
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

    fn calc_state(&self, pos: Position) -> State {
        let mut state = Vec::new();
        let d = self.look_dist as i32;
        for i in -d..d + 1 {
            for j in -d..d + 1 {
                if i.abs() + j.abs() > d || (i == 0 && j == 0) {
                    continue;
                }
                let x = pos.0 as i32 - i;
                let y = pos.1 as i32 - j;
                let x = if x.is_negative() {
                    (x + self.grid[0].len() as i32) as usize
                } else {
                    x as usize % self.grid[0].len()
                };
                let y = if y.is_negative() {
                    (y + self.grid.len() as i32) as usize
                } else {
                    y as usize % self.grid.len()
                };

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

    fn update_mouse_old(&mut self) {
        let state = self.calc_state(self.mouse.pos);

        let mut reward: f64 = -1.0;

        if self.mouse.pos == self.cat.pos && self.cat_enabled {
            self.mouse.eaten += 1;
            reward = -100.0;
            if let (Some(last_state), Some(last_action)) =
                (self.mouse.last_state.clone(), self.mouse.last_action)
            {
                self.mouse.ai.learn(last_state, last_action, reward, state);
            }
            self.mouse.last_state = None;
            self.mouse.last_action = None;
            self.mouse.pos = rand_pos(&self.grid);
            return;
        }

        if self.mouse.pos == self.cheese.pos {
            self.mouse.fed += 1;
            reward = 50.0;
            self.cheese.pos = rand_pos(&self.grid);
            self.mouse.time_to_cheese_stats.push(0);
        } else {
            *self.mouse.time_to_cheese_stats.last_mut().unwrap() += 1;
        }

        if let (Some(last_state), Some(last_action)) =
            (self.mouse.last_state.clone(), self.mouse.last_action)
        {
            self.mouse.ai.learn(last_state, last_action, reward, state);
        }

        let state = self.calc_state(self.mouse.pos);
        let action = self.mouse.ai.choose_action(state.clone());

        self.mouse.last_state = Some(state);
        self.mouse.last_action = Some(action);

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

    fn update_mouse(&mut self) {
        let state = self.calc_state(self.mouse.pos);

        let mut reward: f64 = 0.0;

        if self.mouse.pos == self.cat.pos && self.cat_enabled {
            self.mouse.eaten += 1;
            reward = -100.0;
            if let (Some(last_state), Some(last_action)) =
                (self.mouse.last_state.clone(), self.mouse.last_action)
            {
                self.mouse.ai.learn(last_state, last_action, reward, state);
            }
            self.mouse.last_state = None;
            self.mouse.last_action = None;
            self.mouse.pos = rand_pos(&self.grid);
            return;
        }

        if self.mouse.pos == self.cheese.pos {
            self.mouse.fed += 1;
            reward = 50.0;
            self.cheese.pos = rand_pos(&self.grid);
            self.mouse.time_to_cheese_stats.push(0);
        } else {
            *self.mouse.time_to_cheese_stats.last_mut().unwrap() += 1;
        }

        if self.cat_enabled {
            let cat_dist =
                self.dijkstra
                    .shortest_path(&self.neighbors, self.mouse.pos, self.cat.pos);
            reward += 5.0 * cat_dist as f64;
        }

        let cheese_dist =
            self.dijkstra
                .shortest_path(&self.neighbors, self.mouse.pos, self.cheese.pos);
        if cheese_dist < self.mouse.last_cheese_dist {
            reward += 25.0;
        } else {
            reward -= 25.0;
        }
        // reward -= 2.0 * cheese_dist as f64;

        let mut cells = HashMap::new();
        for c in self.mouse.last_cells.iter() {
            if let Some(n) = cells.get_mut(c) {
                *n += 10;
            } else {
                cells.insert(c, 0);
            }
        }

        let backtrack_penalty = *cells.values().max().unwrap_or(&0) as f64;
        reward -= backtrack_penalty;

        if let (Some(last_state), Some(last_action)) =
            (self.mouse.last_state.clone(), self.mouse.last_action)
        {
            self.mouse.ai.learn(last_state, last_action, reward, state);
        }

        let state = self.calc_state(self.mouse.pos);
        let action = self.mouse.ai.choose_action(state.clone());

        self.mouse.last_state = Some(state);
        self.mouse.last_action = Some(action);
        self.mouse.last_cheese_dist = cheese_dist;
        self.mouse.last_cells.push_front(self.mouse.pos);
        if self.mouse.last_cells.len() == 10 {
            self.mouse.last_cells.pop_back();
        }

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

    fn run(&mut self) -> Result<()> {
        let mut stdout = io::stdout();
        let mut last_tick = Instant::now();
        loop {
            if self.age >= self.skip && self.display {
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
                            _ => write!(stdout, " ")?,
                        }
                    }
                }

                stdout.queue(cursor::MoveToNextLine(1))?;
                stdout.write_fmt(format_args!("age = {}", self.age))?;
                stdout.queue(cursor::MoveToNextLine(1))?;
                stdout.write_fmt(format_args!("fed = {}", self.mouse.fed))?;
                stdout.queue(cursor::MoveToNextLine(1))?;
                stdout.write_fmt(format_args!("eaten = {}", self.mouse.eaten))?;
                stdout.queue(cursor::MoveToNextLine(1))?;
                stdout.flush()?;

                let timeout = self.tick_rate.saturating_sub(last_tick.elapsed());
                if event::poll(timeout)? {
                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            _ => {}
                        }
                    }
                }
            }

            if last_tick.elapsed() >= self.tick_rate || self.age <= self.skip || !self.display {
                self.grid[self.cheese.pos.1][self.cheese.pos.0] = Cell::Empty;
                self.grid[self.mouse.pos.1][self.mouse.pos.0] = Cell::Empty;
                if self.cat_enabled {
                    self.grid[self.cat.pos.1][self.cat.pos.0] = Cell::Empty;
                    self.update_cat();
                }
                self.mouse.fed_stats.push(self.mouse.fed);
                if self.mouse.dumb {
                    self.update_mouse_old()
                } else {
                    self.update_mouse();
                }
                self.grid[self.cheese.pos.1][self.cheese.pos.0] = Cell::Cheese;
                self.grid[self.mouse.pos.1][self.mouse.pos.0] = Cell::Mouse;
                if self.cat_enabled {
                    self.grid[self.cat.pos.1][self.cat.pos.0] = Cell::Cat;
                }
                self.age += 1;
                last_tick = Instant::now();
            }

            if self.age == self.epochs {
                break;
            }
        }
        Ok(())
    }
}

fn plot(name: &str, data: Vec<usize>) -> Result<()> {
    let root = BitMapBackend::new(name, (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        // .caption(name, ("sans-serif", 50).into_font())
        .margin(50)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0usize..data.len(), 0usize..*data.iter().max().unwrap())?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(data.into_iter().enumerate(), &RED))?;

    root.present()?;

    Ok(())
}

fn main() -> Result<()> {
    let matches = command!()
        .arg(arg!(<world> "Path to world file"))
        .arg(arg!(-e --epochs <epochs> "Number of epochs").value_parser(value_parser!(usize)))
        .arg(arg!(-c --cat "Cat enabled"))
        .arg(arg!(-d --dumb "Dumb mouse enabled"))
        .arg(arg!(-v --visualize "Visualize game output"))
        .arg(
            arg!(-s --skip <skip> "Skip showing number of epochs")
                .value_parser(value_parser!(usize)),
        )
        .arg(arg!(-p --plot "Plot stats"))
        .get_matches();
    let world_file = matches.get_one::<String>("world").unwrap();
    let epochs = *matches.get_one::<usize>("epochs").unwrap_or(&100_000);
    let cat = *matches.get_one::<bool>("cat").unwrap_or(&false);
    let dumb_mouse = *matches.get_one::<bool>("dumb").unwrap_or(&false);
    let visualize = *matches.get_one::<bool>("visualize").unwrap_or(&false);
    let skip = *matches.get_one::<usize>("skip").unwrap_or(&0);
    let plot_stats = *matches.get_one::<bool>("plot").unwrap_or(&false);

    let mut world = World::new(
        world_file,
        epochs,
        dumb_mouse,
        skip,
        Duration::from_millis(500),
        cat,
        visualize,
    );

    if world.display {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        io::stdout().execute(cursor::Hide)?;
    }

    world.run()?;

    if world.display {
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        io::stdout().execute(cursor::Show)?;
    }

    println!("age = {}", world.age);
    println!("fed = {}", world.mouse.fed);
    println!("eaten = {}", world.mouse.eaten);

    if plot_stats {
        plot("fed.png", world.mouse.fed_stats)?;
        plot("time_to_cheese.png", world.mouse.time_to_cheese_stats)?;
    }

    Ok(())
}
