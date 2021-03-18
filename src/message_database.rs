use rand::Rng;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Point {
    pub lat: f64,
    pub lon: f64
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct BoundingBox {
    pub nw: Point,
    pub se: Point
}

impl BoundingBox {
    pub fn random() -> BoundingBox {
        let mut rng = rand::thread_rng();
        let mid = Point {
            lat: rng.gen_range(-90.0..90.0),
            lon: rng.gen_range(-180.0..180.0),
        };
        let delta = 1.0;
        let nw = Point {
            lat: mid.lat + delta,
            lon: mid.lon - delta,
        };
        let se = Point {
            lat: mid.lat - delta,
            lon: mid.lon + delta,
        };
        BoundingBox { nw, se }
    }

    pub fn overlap(&self, other: &BoundingBox) -> bool {
        return
            (self.se.lat <= other.nw.lat && other.nw.lat <= self.nw.lat ||
             self.se.lat <= other.se.lat && other.se.lat <= self.nw.lat) &&
            (self.nw.lon <= other.se.lon && other.se.lon <= self.se.lon ||
             self.nw.lon <= other.nw.lon && other.nw.lon <= self.se.lon);
    }
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub created_at: i64, // unix timestamp
    pub user: String,
    pub text: String,
    pub bounding_box: BoundingBox,
}

pub type Messages = Vec<Message>;


#[cfg(test)]
mod tests {
    #[test]
    fn overlap_overlaps() {
        let a = BoundingBox {
            nw: Point {lat: 50, lon:0},
            se: Point {lat: 40, lon:10},
        };
        let b = BoundingBox {
            nw: Point {lat: 49, lon: 2},
            se: Point {lat: 42, lon: 20},
        };
        assert!(a.overlaps(b));
    }
}
