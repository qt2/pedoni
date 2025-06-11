# Pedoni

Blazingly fast crowd simulator for pedestrian dynamics

## Get started
```sh
cargo run -r -- scenarios/narrow-gap.toml
```

## Help
```sh
cargo run -r -- -h
```

## Directory structure
- `/pedoni`: main application / renderer
- `/pedoni-simulator`: simulation logic implementation
- `/scenarios`: example scenarios

## Scenario file syntax
```toml
[field]
# Size of the field, 200 x 200 m
size = [200, 200] 

[[waypoints]]
line = [[10, 10], [10, 190]] # Vertex positions that consist the line segment of a waypoint.

[[waypoints]]
line = [[190, 10], [190, 190]]

[[obstacles]]
line = [[50, 0], [100, 90]]
width = 5

[[obstacles]]
line = [[50, 200], [100, 110]]
width = 5

[[obstacles]]
line = [[150, 0], [100, 90]]
width = 5

[[obstacles]]
line = [[150, 200], [100, 110]]
width = 5

[[pedestrians]]
origin = 0
destination = 1
spawn = { kind = "periodic", frequency = 100.0 }

[[pedestrians]]
origin = 1
destination = 0
spawn = { kind = "periodic", frequency = 100.0 }
```

## License
MIT