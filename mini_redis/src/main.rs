use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt};
use tokio::io::{AsyncBufReadExt, BufReader};
//use tokio::time::{timeout, Duration};
use tokio::sync::Mutex;

use std::collections::HashMap;

use std::sync::Arc;

use serde::{Serialize, Deserialize};
// Structure suggérée pour l'état partagé :
type Store = Arc<Mutex<HashMap<String, String>>>;

// Classe pour récupérer le JSON

#[derive(Serialize, Deserialize, Debug)]
struct Requete {
    cmd: String,
    key: Option<String>,
    value: Option<String>,
    seconds: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Reponse {
    status: String,
    value: Option<String>,
    count: Option<u16>,
    keys: Option<Vec<String>>,
    ttl: Option<u16>,
    message: Option<String>,
}

async fn process_command(line: String, store: Store) -> Option<Reponse>
{
    println!("{}", line);
    let deserialized: Requete = serde_json::from_str(&line).expect("no panik");

    match deserialized.cmd.as_str() {
        "PING" => Some(Reponse {status: "ok".to_string(), value: None, count: None, keys: None, ttl: None, message: None}),
        "SET" => set_command(deserialized.key.expect(""), deserialized.value.expect(""), store).await,
        "GET" => get_command(deserialized.key.expect(""), store).await,
        "DEL" => del_command(deserialized.key.expect(""), store).await,
        _ => Some(Reponse {status: "error".to_string(), value: None, count: None, keys: None, ttl: None, message: Some("function not found".to_string())}),
    }
}

async fn set_command(key: String, value: String, store: Store) -> Option<Reponse> {
    //Je verrouille le Mutex et prends le hashmap
    let mut hashmap = store.lock().await;

    if hashmap.contains_key(&key) {
        *hashmap.get_mut(&key).unwrap() = value;
    }
    else {
        hashmap.insert(key, value);
    }

    Some(Reponse {status: "ok".to_string(), value: None, count: None, keys: None, ttl: None, message: None})
}

async fn get_command(key: String, store: Store) -> Option<Reponse> {
    let hashmap = store.lock().await;

    if hashmap.contains_key(&key) {
        let r_value = hashmap.get(&key).unwrap().clone();
        Some(Reponse { status: "ok".to_string(), value: Some(r_value), count: None, keys: None, ttl: None, message: None})
    }
    else {
        Some(Reponse { status: "ok".to_string(), value: None, count: None, keys: None, ttl: None, message: None})
    }
}

async fn del_command(key: String, store: Store) -> Option<Reponse> {
    let mut hashmap = store.lock().await;

    if hashmap.contains_key(&key) {
        hashmap.remove(&key);
        Some(Reponse { status: "ok".to_string(), value: None, count: Some(1), keys: None, ttl: None, message: None})
    }
    else {
        Some(Reponse { status: "ok".to_string(), value: None, count: Some(0), keys: None, ttl: None, message: None})
    }
}

async fn handle_client(socket: TcpStream, store: Store)
{
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
        
    let _ = reader.read_line(&mut line).await;

    let reponse = process_command(line.trim().to_string(), store.clone()).await;
    let json = serde_json::to_string(&reponse).unwrap();

    write_half.write_all(json.as_bytes()).await.unwrap();
}

#[tokio::main]
async fn main() {
    // Initialiser tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // TODO: Implémenter le serveur MiniRedis sur 127.0.0.1:7878
    //
    // Étapes suggérées :
    // 1. Créer le store partagé (Arc<Mutex<HashMap<String, ...>>>)
    // 2. Bind un TcpListener sur 127.0.0.1:7878
    // 3. Accept loop : pour chaque connexion, spawn une tâche
    // 4. Dans chaque tâche : lire les requêtes JSON ligne par ligne,
    //    traiter la commande, envoyer la réponse JSON + '\n'

    let store: Store = Arc::new(Mutex::new(HashMap::new()));

    // Bind du TCP

    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();

    // Accept loop classique :
    loop {
        let (socket, _addr) = listener.accept().await.unwrap();
        let store = store.clone();
        tokio::spawn(async move {
            handle_client(socket, store).await;
        });
    }

    //println!("MiniRedis - à implémenter !");
}
