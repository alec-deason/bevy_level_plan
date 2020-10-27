This is an experiment in making a high level DSL for specifying the sequence of events that should happen in a game level.

Here's an example from the [example](examples/simple_plan.rs#L15):
```rust
LevelPlan::<ExampleLevelContext>::new(
    Sequence::default()
        .push(ForDistance::new(
            level_length - 1800.0,
            Cycle::new(
                Sequence::default()
                    .push(ForDistance::new(
                        500.0,
                        SetComponent::new(DiverSpawner::default()),
                    ))
                    .push(ForDistance::new(
                        500.0,
                        SetComponent::new(SwooperSpawner::default()),
                    )),
            ),
        ))
        .push(ForDistance::new(
            1000.0,
            Nop
        ))
        .push(Conditional::<ExampleLevelContext>::new(
            move |context| context.player_health < 4,
            SpawnPowerups,
        ))
        .push(While::<ExampleLevelContext>::new(
            |context| context.boss_spawned,
            SpawnBoss,
        ))
        .push(YouWin),
)
```

The `LevelPlan` is a component which can be spawned into the world and then referenced from systems. This example, using a mix of generic `LevelPlanElement` types and some specific to the structure of the example game, says to repeat a cycle two different enemy types each lasting for 500px of vertical travel. Once the player nears the end of the level the plan leaves a dead space where nothing happens for 1000px then if the player is damaged spawns some heals. After that there's a boss encounter which lasts until the boss is no longer spawned. Then a win condition trigger.

The different steps interact with the world by conditioning sub steps, like `ForDistance`, `While` and `Condition`; adding a component to the `LevelPlan` entity, like `SetComponent` does, which can be used to trigger custom, state specific systems; or take custom actions on activation, deactivation or each tick while active as `SpawnBoss` and `SpawnPowerups` do. `LevelPlan` is generic over a context type, `ExampleLevelContext` in this example, which can be used to carry information about the world into plan elements, for example the condition closures on `Condition` and `While`.

Generally the plan is executed by the generic `level_plan_system`.
