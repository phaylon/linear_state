
use linear_state::{RootState, search};

#[derive(Debug, Clone, Eq, PartialEq)]
struct At { place: &'static str }

struct Path { to: &'static str, from: &'static str }

#[derive(Debug, Clone, Eq, PartialEq)]
struct Count { value: u64 }

#[test]
fn basic() {

    let start = RootState::new()
        .with(Count { value: 0 })
        .with(At { place: "A" })
        .with_many(vec![
            Path { from: "A", to: "B" },
            Path { from: "B", to: "C" },
            Path { from: "A", to: "D" },
            Path { from: "D", to: "B" },
            Path { from: "A", to: "F" },
            Path { from: "D", to: "X" },
            Path { from: "X", to: "Y" },
        ])
        .finalize();

    let find_path = |target| search(
        vec![start.clone()],
        |state, collector| {
            state.descend_consumed::<At, _>(|state, at| {
                state.descend_mapped::<Count, _, _>(
                    |prev| Count { value: prev.value + 1 },
                    |state, _| {
                        for path in state.get::<Path>() {
                            if path.from == at.place {
                                collector.push(state.with_produced(Some(At { place: path.to })));
                            }
                        }
                    },
                );
            });
        },
        |state| state.has(&At { place: target }),
        |_| (),
    );

    let solutions = find_path("C");
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get::<At>(), &[At { place: "C" }]);
    assert_eq!(solutions[0].get::<Count>(), &[Count { value: 2 }]);

    let solutions = find_path("Y");
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].get::<At>(), &[At { place: "Y" }]);
    assert_eq!(solutions[0].get::<Count>(), &[Count { value: 3 }]);
}
