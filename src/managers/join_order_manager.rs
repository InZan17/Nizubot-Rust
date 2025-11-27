use std::{
    cmp::Ordering,
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::{futures::StreamExt, GuildId, Http, Timestamp, UserId};
use tokio::sync::{Mutex, RwLock};

use crate::{utils::TtlMap, Error};

#[derive(Debug, Clone)]
pub struct LightweightMember {
    pub id: UserId,
    pub tag: String,
    pub joined_at: Option<Timestamp>,
}

pub struct GuildJoinOrder {
    guild_id: GuildId,
    pending_members: Mutex<Vec<LightweightMember>>,
    sorted_members: Mutex<Vec<LightweightMember>>,
}

impl GuildJoinOrder {
    pub fn new(guild_id: GuildId) -> Self {
        Self {
            guild_id,
            pending_members: Mutex::new(Vec::new()),
            sorted_members: Mutex::new(Vec::new()),
        }
    }

    pub async fn insert_member(&self, added_member: LightweightMember) {
        self.pending_members.lock().await.push(added_member);
    }

    pub async fn remove_member(&self, removed_user_id: UserId) {
        let mut sorted_members = self.sorted_members.lock().await;
        sorted_members.retain(|member| member.id != removed_user_id);
        let mut pending_members = self.pending_members.lock().await;
        pending_members.retain(|member| member.id != removed_user_id);
        drop(sorted_members);
        drop(pending_members);
    }

    pub async fn update_member(&self, new_member: LightweightMember) {
        let mut sorted_members = self.sorted_members.lock().await;
        if let Some(member) = sorted_members
            .iter_mut()
            .find(|old_member| old_member.id == new_member.id)
        {
            *member = new_member.clone()
        };
        let mut pending_members = self.pending_members.lock().await;
        if let Some(member) = pending_members
            .iter_mut()
            .find(|old_member| old_member.id == new_member.id)
        {
            *member = new_member.clone()
        };
        drop(sorted_members);
        drop(pending_members);
    }

    pub async fn get_sorted_members(
        &self,
        member_count: u64,
        http: &Http,
    ) -> Result<
        (
            tokio::sync::MutexGuard<'_, Vec<LightweightMember>>,
            u32,
            u128,
            Option<u128>,
        ),
        Error,
    > {
        let mut sorted_members = self.sorted_members.lock().await;
        let mut pending_members = self.pending_members.lock().await;
        let remove_ids = pending_members
            .iter()
            .map(|member| member.id)
            .collect::<Vec<_>>();

        sorted_members.retain(|member| !remove_ids.contains(&member.id));

        sorted_members.append(&mut pending_members);

        if sorted_members.len() as u64 == member_count {
            let mut comparisons = 0_u32;
            let now = get_current_ms_time();

            sorted_members.sort_by(|a, b| {
                comparisons += 1;
                cmp_joined_at_opt(a.joined_at, b.joined_at)
            });

            let sort_ms = get_current_ms_time() - now;

            return Ok((sorted_members, comparisons, sort_ms, None));
        }

        // Re-fetch all members.
        println!("Re-fetching all members");
        drop(pending_members);
        *sorted_members = vec![];

        let mut new_members: HashMap<UserId, LightweightMember> = HashMap::new();

        let now = get_current_ms_time();
        let mut members_iter = self.guild_id.members_iter(http).boxed();

        while let Some(member) = members_iter.next().await {
            let member = member?;

            let fetched_member = LightweightMember {
                id: member.user.id,
                tag: member.user.tag(),
                joined_at: member.joined_at,
            };

            let entry = new_members.entry(fetched_member.id);

            entry
                .and_modify(|member| {
                    if cmp_joined_at_opt(member.joined_at, fetched_member.joined_at)
                        == Ordering::Less
                    {
                        *member = fetched_member.clone()
                    }
                })
                .or_insert(fetched_member);
        }

        let mut new_members = new_members.into_iter().map(|(_, v)| v).collect::<Vec<_>>();

        let mut pending_members = self.pending_members.lock().await;
        let remove_ids = pending_members
            .iter()
            .map(|member| member.id)
            .collect::<Vec<_>>();

        new_members.retain(|member| !remove_ids.contains(&member.id));

        new_members.append(&mut pending_members);

        *sorted_members = new_members;

        let fetch_ms = get_current_ms_time() - now;

        let mut comparisons = 0_u32;
        let now = get_current_ms_time();

        sorted_members.sort_by(|a, b| {
            comparisons += 1;
            cmp_joined_at_opt(a.joined_at, b.joined_at)
        });

        let sort_ms = get_current_ms_time() - now;

        return Ok((sorted_members, comparisons, sort_ms, Some(fetch_ms)));
    }

    pub async fn get_members_around_user_or_index(
        &self,
        member_count: u64,
        target_user_id: UserId,
        index: Option<usize>,
        http: &Http,
    ) -> Result<
        (
            Vec<(usize, LightweightMember)>,
            usize,
            u32,
            u128,
            Option<u128>,
        ),
        Error,
    > {
        let (members, comparisons, sort_ms, fetch_ms) =
            self.get_sorted_members(member_count, http).await?;

        let mut target_index = 0;

        if let Some(index) = index {
            target_index = index;
        } else {
            for (index, member) in members.iter().enumerate() {
                if member.id == target_user_id {
                    target_index = index;
                    break;
                }
            }
        }

        let members_len = members.len();
        if members_len == 0 {
            return Ok((vec![], 0, comparisons, sort_ms, fetch_ms));
        };
        let max_possible_index = members_len - 1;

        let mut max_index = max_possible_index.min(target_index + 4);
        let mut min_index = target_index.saturating_sub(4);

        if min_index == 0 {
            max_index = 8.min(max_possible_index);
        } else if max_index == max_possible_index {
            min_index = max_possible_index.saturating_sub(8);
        };

        let mut members_around = Vec::with_capacity((max_index - min_index + 1) as usize);

        for i in min_index..max_index + 1 {
            members_around.push((i, members[i].clone()));
        }

        return Ok((members_around, target_index, comparisons, sort_ms, fetch_ms));
    }
}

pub struct JoinOrderManager {
    pub join_orders: RwLock<TtlMap<GuildId, Arc<GuildJoinOrder>>>,
}

impl JoinOrderManager {
    pub fn new() -> Self {
        Self {
            join_orders: RwLock::new(TtlMap::new(Duration::from_secs(60 * 60))),
        }
    }

    pub async fn get_join_order(&self, guild_id: GuildId) -> Arc<GuildJoinOrder> {
        if let Some(join_order) = self.join_orders.read().await.get(&guild_id).cloned() {
            return join_order;
        }

        let mut join_orders_mut = self.join_orders.write().await;
        if let Some(join_order) = join_orders_mut.get(&guild_id).cloned() {
            return join_order;
        }

        let join_order = Arc::new(GuildJoinOrder::new(guild_id));

        join_orders_mut.insert(guild_id, join_order.clone());

        join_order
    }

    pub async fn silent_get_join_order(&self, guild_id: GuildId) -> Option<Arc<GuildJoinOrder>> {
        self.join_orders.read().await.get_silent(&guild_id).cloned()
    }
}

fn cmp_joined_at_opt(joined_at_a: Option<Timestamp>, joined_at_b: Option<Timestamp>) -> Ordering {
    match (joined_at_a, joined_at_b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(timestamp_a), Some(timestamp_b)) => timestamp_a.cmp(&timestamp_b),
    }
}

pub fn join_order_manager_loop(join_order_manager: Arc<JoinOrderManager>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            join_order_manager.join_orders.write().await.clear_expired();
        }
    });
}

pub fn get_current_ms_time() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards. Oopsie.");
    since_the_epoch.as_millis()
}
