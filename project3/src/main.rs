use num_bigint::BigUint;
use plotters::{
    coord::types::RangedCoordf32,
    prelude::{
        BitMapBackend, Cartesian2d, ChartBuilder, Circle, DrawingArea, EmptyElement,
        IntoDrawingArea, IntoSegmentedCoord, Text,
    },
    series::Histogram,
    style::{Color, IntoFont, ShapeStyle, BLACK, RED, WHITE},
};
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::{
    collections::HashMap,
    error::Error,
    f32::consts::PI,
    fmt::{self, Debug},
};

const M: u64 = 32;

#[derive(Clone, PartialEq, Eq)]
struct Node {
    name: String,
    host: String,
    hash: u64,
}

impl Node {
    fn new(name: String, host: String) -> Self {
        let hash = hash_sha256(&host);
        Self { name, host, hash }
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node {{name: {}, hash: {}}}", self.name, self.hash)
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
    let hash = Sha256::digest(input.as_bytes());
    let int = BigUint::from_bytes_be(&hash);
    (int % M).try_into().unwrap()
}

fn hash_sha384(input: &str) -> u64 {
    let hash = Sha384::digest(input.as_bytes());
    let int = BigUint::from_bytes_be(&hash);
    (int % M).try_into().unwrap()
}

fn hash_sha512(input: &str) -> u64 {
    let hash = Sha512::digest(input.as_bytes());
    let int = BigUint::from_bytes_be(&hash);
    (int % M).try_into().unwrap()
}

fn compute_finger_tables(nodes: &[Node]) -> HashMap<String, Vec<Node>> {
    let mut finger_tables = HashMap::new();
    for node in nodes {
        let mut finger_table = Vec::new();
        let hashes: Vec<u64> = (0..5).map(|i| (node.hash + 2u64.pow(i)) % M).collect();
        for hash in hashes {
            let n = nodes
                .iter()
                .find(|n| n.hash >= hash)
                .unwrap_or(nodes.first().unwrap())
                .clone();
            finger_table.push(n);
        }

        finger_tables.insert(node.name.clone(), finger_table);
    }
    finger_tables
}

fn successor(finger_table: &[Node], key: u64) -> &Node {
    let mut tmp = finger_table.to_vec();
    let mut min = u64::MAX;
    let mut successor = finger_table.first().unwrap();
    for (i, node) in finger_table.iter().enumerate() {
        if node.hash < key {
            tmp[i].hash += M;
        }

        let d = tmp[i].hash - key;

        if d < min {
            min = d;
            successor = node;
        }
    }
    successor
}

fn predecessor(finger_table: &[Node], key: u64) -> &Node {
    let mut min = u64::MAX;
    let mut predecessor = finger_table.first().unwrap();
    for node in finger_table {
        let mut tmp_key = key;
        if key < node.hash {
            tmp_key += M;
        }

        let d = tmp_key - node.hash;

        if d < min && d != 0 {
            min = d;
            predecessor = node;
        }
    }
    predecessor
}

fn route_to_key(
    finger_tables: &HashMap<String, Vec<Node>>,
    init_node: &str,
    key: u64,
) -> Vec<Node> {
    let mut table = finger_tables.get(init_node).unwrap();
    let mut node = table.first().unwrap();
    for ftab in finger_tables.values() {
        if let Some(n) = ftab.iter().find(|n| n.name == init_node) {
            node = n;
            break;
        }
    }

    let mut route = vec![node.clone()];

    loop {
        let successor = successor(table, key);
        let first = table.first().unwrap();
        let mut tmp_key = key;
        let mut tmp_succ_hash = successor.hash;

        if key < node.hash {
            tmp_key += M;
        }

        if successor.hash < node.hash {
            tmp_succ_hash += M;
        }

        if *successor == *first && node.hash < tmp_key && tmp_succ_hash >= tmp_key {
            route.push(successor.clone());
            return route;
        }

        node = predecessor(table, key);
        route.push(node.clone());
        table = finger_tables.get(&route.last().unwrap().name).unwrap();
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

fn plot_loads(node_files: HashMap<u64, usize>, n: usize) -> Result<(), Box<dyn Error>> {
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
        Node::new("A".to_string(), "239.67.52.72".to_string()),
        Node::new("B".to_string(), "137.70.131.229".to_string()),
        Node::new("C".to_string(), "98.5.87.182".to_string()),
        Node::new("D".to_string(), "11.225.158.95".to_string()),
        Node::new("E".to_string(), "203.187.116.210".to_string()),
        Node::new("F".to_string(), "107.117.238.203".to_string()),
        Node::new("G".to_string(), "27.161.219.131".to_string()),
    ];

    let orig_nodes = nodes.clone();

    println!("zero hash = {}", hash_sha256(""));

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

    let root = BitMapBackend::new("ring.png", (1000, 1000)).into_drawing_area();
    root.fill(&WHITE)?;

    let root = root.apply_coord_spec(Cartesian2d::<RangedCoordf32, RangedCoordf32>::new(
        -500f32..500f32,
        -500f32..500f32,
        (0..1000, 0..1000),
    ));

    root.draw(&Circle::new((0f32, 0f32), 250, ShapeStyle::from(BLACK)))?;
    let points = map_points_on_circle(M as usize, 250f32);

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
    println!("-------------------------------------------------------");

    let finger_tables = compute_finger_tables(&nodes);
    println!("finger tables:");
    for table in finger_tables.iter() {
        println!("{table:?}");
    }

    // -------------------- 3 --------------------
    println!("-------------------------------------------------------");

    for f in files {
        println!(
            "file = {} key = {} route = {:?}",
            f.name,
            f.hash,
            route_to_key(&finger_tables, "A", f.hash)
        );
    }

    // -------------------- 4 --------------------
    println!("-------------------------------------------------------");
    let mut nodes = nodes.to_vec();
    println!("node hashes for 1 hash:");

    for node in nodes.iter() {
        println!("node {} hash {}", node.name, node.hash);
    }
    let mut node_files: HashMap<u64, usize> = HashMap::new();
    for key in (0..1000).map(|i| hash_sha256(&format!("f{i}.mov"))) {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= key)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        *node_files.entry(node_id).or_default() += 1;
    }
    println!("files per node = {node_files:?}");
    plot_loads(node_files, 1)?;

    // -------------------- 2 hashes --------------------
    println!("node hashes for 2 hash:");
    for node in orig_nodes.iter() {
        let mut new_hashed = node.clone();
        new_hashed.hash = hash_sha384(&new_hashed.host);
        // hash already used, fined next free hash
        if nodes.iter().any(|n| n.hash == new_hashed.hash) {
            for j in 0..M {
                if !nodes.iter().any(|n| n.hash == (new_hashed.hash + j) % M) {
                    new_hashed.hash = (new_hashed.hash + j) % M;
                    break;
                }
            }
        }
        nodes.push(new_hashed);
    }

    nodes.sort_by_key(|n| n.hash);

    for node in nodes.iter() {
        println!("node {} hash {}", node.name, node.hash);
    }

    let mut node_files: HashMap<u64, usize> = HashMap::new();
    for key in (0..1000).map(|i| hash_sha256(&format!("f{i}.mov"))) {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= key)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        *node_files.entry(node_id).or_default() += 1;
    }
    println!("files per node = {node_files:?}");
    plot_loads(node_files, 2)?;

    // -------------------- 3 hashes --------------------
    println!("node hashes for 3 hash:");
    for node in orig_nodes.iter() {
        let mut new_hashed = node.clone();
        new_hashed.hash = hash_sha512(&new_hashed.host);
        // hash already used, fined next free hash
        if nodes.iter().any(|n| n.hash == new_hashed.hash) {
            for j in 0..M {
                if !nodes.iter().any(|n| n.hash == (new_hashed.hash + j) % M) {
                    new_hashed.hash = (new_hashed.hash + j) % M;
                    break;
                }
            }
        }
        nodes.push(new_hashed);
    }

    nodes.sort_by_key(|n| n.hash);

    for node in nodes.iter() {
        println!("node {} hash {}", node.name, node.hash);
    }

    let mut node_files: HashMap<u64, usize> = HashMap::new();
    for key in (0..1000).map(|i| hash_sha256(&format!("f{i}.mov"))) {
        let node_id = nodes
            .iter()
            .find(|n| n.hash >= key)
            .unwrap_or(nodes.first().unwrap())
            .hash;
        *node_files.entry(node_id).or_default() += 1;
    }
    println!("files per node = {node_files:?}");
    plot_loads(node_files, 3)?;

    Ok(())
}
