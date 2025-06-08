use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_observer(give_ammo)
        .register_type::<HasLimitedAmmo>();
}
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct HasLimitedAmmo(pub i32);
impl HasLimitedAmmo {
    pub fn can_shoot(&self) -> bool {
        self.0 > 0
    }
}

#[derive(Event, Debug)]
pub struct GiveAmmo(pub i32);

fn give_ammo(trigger: Trigger<GiveAmmo>, mut query: Query<&mut HasLimitedAmmo>) {
    let Ok(mut e) = query.get_mut(trigger.target()) else {
        info!(
            "Tried to give ammo to entity that doesn't track it! {:?}",
            trigger
        );
        return;
    };

    let amount = trigger.0;
    e.0 += amount;
    
    if amount < 0 {
        info!("Deducting {:?} ammos", amount);
    } else {
        info!("Adding {:?} ammos", amount);
    }
}
