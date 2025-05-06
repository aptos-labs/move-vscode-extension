module 0x1::schema_includes_lit {
    spec module {
        include true ==> MySchema;
        include vote.agree != agree ==> CheckChangeVote<TokenT, ActionT>{vote, proposer_address};
    }
}
