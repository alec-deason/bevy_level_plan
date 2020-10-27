use bevy::prelude::*;

pub trait LevelContext {
    fn build(world: &World, resources: &Resources) -> Self;
}
pub fn level_progress_system<T: LevelContext + 'static>(
    world: &mut World,
    resources: &mut Resources,
) {
    let mut context = T::build(world, resources);
    let mut command_buffer = Commands::default();
    command_buffer.set_entity_reserver(world.get_entity_reserver());
    for (entity, mut plan) in &mut world.query_mut::<(Entity, &mut LevelPlan<T>)>().iter() {
        if !plan.activated {
            plan.plan
                .activate(entity, &mut command_buffer, &mut context);
            plan.activated = true;
        }
        if !plan.plan.step(entity, &mut command_buffer, &mut context) {
            command_buffer.despawn(entity);
        }
    }
    command_buffer.apply(world, resources);
}

pub struct LevelPlan<T> {
    plan: Box<dyn LevelPlanElement<T>>,
    activated: bool,
}
impl<T> LevelPlan<T> {
    pub fn new(element: impl LevelPlanElement<T> + 'static) -> Self {
        Self {
            plan: Box::new(element),
            activated: false,
        }
    }
}

pub trait LevelPlanElement<T>: Send + Sync {
    fn step(&mut self, _level: Entity, _commands: &mut Commands, _context: &mut T) -> bool {
        true
    }
    fn activate(&mut self, _level: Entity, _commands: &mut Commands, _context: &mut T) {}
    fn deactivate(&mut self, _level: Entity, _commands: &mut Commands, _context: &mut T) {}
}

pub struct Sequence<T> {
    index: usize,
    elements: Vec<Box<dyn LevelPlanElement<T>>>,
}
impl<T> Default for Sequence<T> {
    fn default() -> Self {
        Self {
            index: 0,
            elements: Vec::new()
        }
    }
}
impl<T> Sequence<T> {
    pub fn push(mut self, element: impl LevelPlanElement<T> + 'static) -> Self {
        self.elements.push(Box::new(element));
        self
    }
}
impl<T> LevelPlanElement<T> for Sequence<T> {
    fn step(&mut self, level: Entity, commands: &mut Commands, context: &mut T) -> bool {
        if let Some(element) = self.elements.get_mut(self.index) {
            if !element.step(level, commands, context) {
                self.elements[self.index].deactivate(level, commands, context);
                self.index += 1;
                if self.index < self.elements.len() {
                    self.elements[self.index].activate(level, commands, context);
                    true
                } else {
                    false
                }
            } else {
                true
            }
        } else {
            false
        }
    }

    fn activate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        self.index = 0;
        self.elements[0].activate(level, commands, context);
    }

    fn deactivate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        self.elements[self.index].deactivate(level, commands, context);
        self.index = 0;
    }
}

pub struct While<T> {
    condition: Box<dyn Fn(&T) -> bool + Send + Sync + 'static>,
    element: Box<dyn LevelPlanElement<T>>,
}
impl<T> While<T> {
    pub fn new(
        condition: impl Fn(&T) -> bool + Send + Sync + 'static,
        element: impl LevelPlanElement<T> + 'static,
    ) -> Self {
        Self {
            condition: Box::new(condition),
            element: Box::new(element),
        }
    }
}
impl<T> LevelPlanElement<T> for While<T> {
    fn step(&mut self, level: Entity, commands: &mut Commands, context: &mut T) -> bool {
        if (self.condition)(context) {
            self.element.step(level, commands, context)
        } else {
            false
        }
    }

    fn activate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        self.element.activate(level, commands, context);
    }

    fn deactivate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        self.element.deactivate(level, commands, context);
    }
}

pub struct Cycle<T> {
    sequence: Sequence<T>,
}
impl<T> Cycle<T> {
    pub fn new(sequence: Sequence<T>) -> Self {
        Self {
            sequence,
        }
    }
}
impl<T> LevelPlanElement<T> for Cycle<T> {
    fn step(&mut self, level: Entity, commands: &mut Commands, context: &mut T) -> bool {
        if !self.sequence.step(level, commands, context) {
            self.sequence.activate(level, commands, context);
        }
        true
    }

    fn activate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        self.sequence.activate(level, commands, context);
    }

    fn deactivate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        self.sequence.deactivate(level, commands, context);
    }
}

pub struct Nop;
impl<T> LevelPlanElement<T> for Nop {}

pub struct SetComponent<C> {
    component: C,
}
impl<C> SetComponent<C> {
    pub fn new(component: C) -> Self {
        Self { component }
    }
}
impl<T, C: Send + Sync + Clone + 'static> LevelPlanElement<T> for SetComponent<C> {
    fn activate(&mut self, level: Entity, commands: &mut Commands, _context: &mut T) {
        commands.insert_one(level, self.component.clone());
    }

    fn deactivate(&mut self, level: Entity, commands: &mut Commands, _context: &mut T) {
        commands.remove_one::<C>(level);
    }
}

pub struct Conditional<T> {
    condition: Box<dyn Fn(&T) -> bool + Send + Sync + 'static>,
    if_branch: Box<dyn LevelPlanElement<T>>,
    if_active: bool,
    else_branch: Option<Box<dyn LevelPlanElement<T>>>,
    else_active: bool,
}
impl<T> Conditional<T> {
    pub fn new(
        condition: impl Fn(&T) -> bool + Send + Sync + 'static,
        if_branch: impl LevelPlanElement<T> + 'static,
    ) -> Self {
        Self {
            condition: Box::new(condition),
            if_branch: Box::new(if_branch),
            if_active: false,
            else_branch: None,
            else_active: false,
        }
    }

    pub fn if_else(
        condition: impl Fn(&T) -> bool + Send + Sync + 'static,
        if_branch: impl LevelPlanElement<T> + 'static,
        else_branch: impl LevelPlanElement<T> + 'static,
    ) -> Self {
        Self {
            condition: Box::new(condition),
            if_branch: Box::new(if_branch),
            if_active: false,
            else_branch: Some(Box::new(else_branch)),
            else_active: false,
        }
    }
}
impl<T> LevelPlanElement<T> for Conditional<T> {
    fn step(&mut self, level: Entity, commands: &mut Commands, context: &mut T) -> bool {
        if (self.condition)(context) {
            if !self.if_active {
                self.if_branch.activate(level, commands, context);
                self.if_active = true;
            }
            if self.else_active {
                if let Some(else_branch) = self.else_branch.as_mut() {
                    else_branch.deactivate(level, commands, context);
                }
                self.else_active = false;
            }
            self.if_branch.step(level, commands, context)
        } else {
            if self.if_active {
                self.if_branch.deactivate(level, commands, context);
                self.if_active = false;
            }
            if let Some(else_branch) = self.else_branch.as_mut() {
                if !self.else_active {
                    else_branch.activate(level, commands, context);
                    self.else_active = true;
                }
                else_branch.step(level, commands, context)
            } else {
                false
            }
        }
    }

    fn deactivate(&mut self, level: Entity, commands: &mut Commands, context: &mut T) {
        if self.if_active {
            self.if_branch.deactivate(level, commands, context);
            self.if_active = false;
        }
        if self.else_active {
            if let Some(else_branch) = self.else_branch.as_mut() {
                else_branch.deactivate(level, commands, context);
            }
            self.else_active = false;
        }
    }
}
