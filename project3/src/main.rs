use std::{
    collections::{HashMap, HashSet},
    error::Error,
    f32::consts::PI,
};

use num_bigint::BigUint;
use plotters::{
    coord::types::{RangedCoordf32, RangedCoordu32},
    prelude::{
        BitMapBackend, Cartesian2d, ChartBuilder, Circle, DrawingArea, EmptyElement,
        IntoDrawingArea, IntoSegmentedCoord, Text,
    },
    series::Histogram,
    style::{Color, IntoFont, ShapeStyle, BLACK, RED, WHITE},
};
use rand::Rng;
use sha2::{Digest, Sha256, Sha384, Sha512};

const M: usize = 25;

#[derive(Debug, Clone)]
struct StorageNode {
    name: String,
    host: String,
    hash: u64,
}

impl StorageNode {
    fn new(name: String, host: String) -> Self {
        let hash = hash_sha256(&host);
        Self { name, host, hash }
    }
}

#[derive(Debug, Clone)]
struct File {
    name: String,
    hash: u64,
}

impl File {
    fn new(name: String) -> Self {
        let hash = hash_sha256(&name);
        Self { name, hash }
    }
}

fn hash_sha256(input: &str) -> u64 {
    let hash = Sha256::digest(input);
    let int = BigUint::from_bytes_le(&hash);
    (int % M).try_into().unwrap()
}

fn hash_sha384(input: &str) -> u64 {
    let hash = Sha384::digest(input);
    let int = BigUint::from_bytes_le(&hash);
    (int % M).try_into().unwrap()
}

fn hash_sha512(input: &str) -> u64 {
    let hash = Sha512::digest(input);
    let int = BigUint::from_bytes_le(&hash);
    (int % M).try_into().unwrap()
}

fn compute_finger_tables(nodes: &[StorageNode]) -> HashMap<String, Vec<StorageNode>> {
    let mut finger_tables = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let finger_table: Vec<_> = nodes.iter().cycle().skip(i + 1).take(5).cloned().collect();
        finger_tables.insert(node.name.clone(), finger_table);
    }
    finger_tables
}

fn route_to_key(
    finger_tables: &HashMap<String, Vec<StorageNode>>,
    init_node: &str,
    key: u64,
) -> Vec<String> {
    let mut route = vec![init_node.to_string()];
    let mut table = finger_tables.get(init_node).unwrap();
    loop {
        if let Some(i) = table.iter().rposition(|n| n.hash <= key) {
            let node = table.get(i).unwrap();
            for t in table.iter() {
                if t.hash >= key {
                    println!("pushed {}", t.name);
                    route.push(t.name.clone());
                    return route;
                }
            }
            println!("pushed {}", node.name);
            route.push(node.name.clone());
            table = finger_tables.get(&node.name).unwrap();
        } else {
            route.push(table.first().unwrap().name.clone());
            table = finger_tables.get(&table.first().unwrap().name).unwrap();
        }
    }
}

fn map_points_on_circle(n: usize, r: f32) -> Vec<(f32, f32)> {
    let mut points = Vec::with_capacity(n);

    for i in 0..n {
        let theta = 2.0 * PI * (i as f32) / (n as f32);
        let x = r * theta.cos();
        let y = r * theta.sin();

        points.push((x, y));
    }

    points
}

fn draw_event(
    root: &DrawingArea<BitMapBackend, Cartesian2d<RangedCoordf32, RangedCoordf32>>,
    label: String,
    x: f32,
    y: f32,
    style: ShapeStyle,
) -> Result<(), Box<dyn Error>> {
    let len = (x.powf(2f32) + y.powf(2f32)).sqrt();
    let l_x = ((x / len) * 50f32) as i32 + if x.is_sign_positive() { -10 } else { 10 };
    let l_y = ((y / len) * 50f32) as i32 + if y.is_sign_positive() { -10 } else { 10 };
    let event = EmptyElement::at((x, y))
        + Circle::new((0, 0), 5, style)
        + Text::new(label, (l_x, l_y), ("sans-serif", 20).into_font());
    root.draw(&event)?;
    Ok(())
}

fn plot_loads(node_files: HashMap<usize, usize>, n: usize) -> Result<(), Box<dyn Error>> {
    let name = format!("loads_{n}.png");
    let root = BitMapBackend::new(&name, (800, 600)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .build_cartesian_2d((0u32..25u32).into_segmented(), 0u32..300u32)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_labels(25)
        .bold_line_style(WHITE)
        .y_desc("Number of keys per node")
        .x_desc("Nodes")
        .axis_desc_style(("sans-serif", 15))
        .draw()?;

    let data: Vec<u32> = (0..M)
        .flat_map(|x| vec![x as u32; *node_files.get(&x).unwrap_or(&0)])
        .collect();

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(RED.filled())
            .data(data.iter().map(|x| (*x, 1))),
    )?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // -------------------- 1 --------------------
    let mut nodes = [
        StorageNode::new("A".to_string(), "239.67.52.72".to_string()),
        StorageNode::new("B".to_string(), "137.70.131.229".to_string()),
        StorageNode::new("C".to_string(), "98.5.87.182".to_string()),
        StorageNode::new("D".to_string(), "11.225.158.95".to_string()),
        StorageNode::new("E".to_string(), "203.187.116.210".to_string()),
        StorageNode::new("F".to_string(), "107.117.238.203".to_string()),
        StorageNode::new("G".to_string(), "27.161.219.131".to_string()),
    ];

    nodes.iter().for_each(|node| {
        println!("node {} has hash {}", node.name, node.hash);
    });

    nodes.sort_by_key(|n| n.hash);

    let num_files = 15;
    let files: Vec<File> = (0..num_files)
        .map(|i| File::new(format!("f{i}.mov")))
        .collect();

    files.iter().for_each(|file| {
        println!("file {} has hash {}", file.name, file.hash);
    });

    let root = BitMapBackend::new("out.png", (1000, 1000)).into_drawing_area();
    root.fill(&WHITE)?;

    let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
        -500f32..500f32,
        -500f32..500f32,
        (0..1000, 0..1000),
    ));

    root.draw(&Circle::new((0f32, 0f32), 300, ShapeStyle::from(BLACK)))?;
    let points = map_points_on_circle(M, 300f32);

    let mut node_files = HashMap::new();
    for file in files.iter() {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= file.hash)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        node_files
            .entry(node_id)
            .or_insert(Vec::new())
            .push(file.name.clone());
    }

    for node in nodes.iter() {
        let p = points.get(node.hash as usize).unwrap();
        draw_event(
            &root,
            format!(
                "{} ({}) {:?}",
                node.name,
                node.hash,
                node_files.get(&node.hash).unwrap_or(&Vec::new())
            ),
            p.0,
            p.1,
            ShapeStyle::from(BLACK).filled(),
        )?;
    }

    root.present()?;

    // -------------------- 2 --------------------

    let finger_tables = compute_finger_tables(&nodes);
    println!("finger tables:");
    for table in finger_tables.iter() {
        println!("{table:?}");
    }

    // -------------------- 3 --------------------

    println!("route to key = {:?}", route_to_key(&finger_tables, "A", 22));

    // -------------------- 4 --------------------
    let mut nodes = nodes.to_vec();
    println!("node hashes for 1 hash:");

    for node in nodes.iter() {
        println!("node {} hash {}", node.name, node.hash);
    }
    let mut rng = rand::thread_rng();
    let mut node_files: HashMap<usize, usize> = HashMap::new();
    for key in (0..1000).map(|_| rng.gen_range(0..M)) {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= key as u64)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        *node_files.entry(node_id as usize).or_default() += 1;
    }
    plot_loads(node_files, 1)?;

    println!("node hashes for 2 hash:");
    for i in 0..7 {
        let mut new_hashed = nodes[i].clone();
        new_hashed.hash = hash_sha384(&new_hashed.host);
        nodes.push(new_hashed);
    }

    nodes.sort_by_key(|n| n.hash);

    for node in nodes.iter() {
        println!("node {} hash {}", node.name, node.hash);
    }

    let mut node_files: HashMap<usize, usize> = HashMap::new();
    for key in (0..1000).map(|_| rng.gen_range(0..M)) {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= key as u64)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        *node_files.entry(node_id as usize).or_default() += 1;
    }
    plot_loads(node_files, 2)?;

    println!("node hashes for 3 hash:");
    for i in 0..7 {
        let mut new_hashed = nodes[i].clone();
        new_hashed.hash = hash_sha512(&new_hashed.host);
        nodes.push(new_hashed);
    }

    nodes.sort_by_key(|n| n.hash);

    for node in nodes.iter() {
        println!("node {} hash {}", node.name, node.hash);
    }

    let mut node_files: HashMap<usize, usize> = HashMap::new();
    for key in (0..1000).map(|_| rng.gen_range(0..M)) {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= key as u64)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        *node_files.entry(node_id as usize).or_default() += 1;
    }
    plot_loads(node_files, 3)?;

    Ok(())
}
