module 0x1::schema_includes_lit {
    spec module {
        include vote.agree != agree ==> CheckChangeVote<TokenT, ActionT>{vote, proposer_address};
    }
}
