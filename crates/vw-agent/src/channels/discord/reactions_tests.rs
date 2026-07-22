use super::*;

#[test]
fn encode_emoji_percent_encodes_unicode_and_ascii_bytes() {
    assert_eq!(encode_emoji_for_discord("A"), "%41");
    assert_eq!(encode_emoji_for_discord("👀"), "%F0%9F%91%80");
    assert_eq!(encode_emoji_for_discord("⚡️"), "%E2%9A%A1%EF%B8%8F");
}

#[test]
fn encode_emoji_leaves_custom_discord_emoji_unchanged() {
    assert_eq!(encode_emoji_for_discord("party_blob:123456"), "party_blob:123456");
}

#[test]
fn encode_emoji_handles_empty_input() {
    assert_eq!(encode_emoji_for_discord(""), "");
}

#[test]
fn reaction_url_strips_discord_prefix_and_encodes_emoji() {
    assert_eq!(
        discord_reaction_url("channel-1", "discord_message-1", "👌"),
        "https://discord.com/api/v10/channels/channel-1/messages/message-1/reactions/%F0%9F%91%8C/@me"
    );
    assert_eq!(
        discord_reaction_url("channel-1", "message-2", "custom:42"),
        "https://discord.com/api/v10/channels/channel-1/messages/message-2/reactions/custom:42/@me"
    );
}

#[test]
fn reaction_url_with_base_trims_trailing_slashes() {
    assert_eq!(
        discord_reaction_url_with_base("http://127.0.0.1:9///", "channel-1", "message-1", "👀"),
        "http://127.0.0.1:9/channels/channel-1/messages/message-1/reactions/%F0%9F%91%80/@me"
    );
}

#[test]
fn random_ack_reaction_is_always_from_static_pool() {
    let pool = discord_ack_reactions();

    assert!(!pool.is_empty());
    for _ in 0..128 {
        assert!(pool.contains(&random_discord_ack_reaction()));
    }
}

#[test]
fn uniform_index_for_single_element_is_always_zero() {
    for _ in 0..32 {
        assert_eq!(pick_uniform_index(1), 0);
    }
}
