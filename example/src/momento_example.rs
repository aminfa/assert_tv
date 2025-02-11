use anyhow::bail;
use serde_json::json;
use assert_tv::{tv_const, TestVectorMomento};
use crate::momento_example::foreign::Point;

mod foreign {
    use rand::distr::{Distribution, StandardUniform};
    use rand::Rng;

    pub struct Point {pub x: u32, pub y: u32}
    impl Distribution<Point> for StandardUniform {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Point {
            Point {
                x: rng.random(),
                y: rng.random()
            }
        }
    }
}

fn display_point_randomly(
    p: &mut Point
) {
    let displacement: Point = rand::random();

    let displacement = tv_const!(displacement, PointMomento);

    p.x = p.x.overflowing_add(displacement.x).0;
    p.y = p.y.overflowing_add(displacement.y).0;
}

struct PointMomento;

impl TestVectorMomento<Point> for PointMomento {
    fn serialize(original_value: &Point) -> anyhow::Result<serde_json::value::Value> {
        Ok(json!({
            "x": original_value.x,
            "y": original_value.y,
        }))
    }

    fn deserialize(value: &serde_json::value::Value) -> anyhow::Result<Point> {
        let Some(map) = value.as_object() else {
            bail!("expected an object")
        };
        let Some(Some(x)) = map.get("x").map(|y| y.as_u64()) else {
            bail!("field x is missing")
        };
        let Some(Some(y)) = map.get("y").map(|y| y.as_u64()) else {
            bail!("field y is missing")
        };
        Ok(Point{
            x: x as u32,
            y: y as u32
        })
    }
}