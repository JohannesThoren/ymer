use ymer_runtime::prelude::*;
use ymer_runtime::{demo, init_logging};

fn main() -> anyhow::Result<()> {
    init_logging();

    App::new(AppConfig {
        title: "sandbox – milstolpe 2".into(),
        width: 1280,
        height: 720,
    })
    .with_setup(|world, _renderer, _assets| demo::setup(world))
    .add_systems((demo::spin_system, propagate_transforms).chain())
    .run()
}
