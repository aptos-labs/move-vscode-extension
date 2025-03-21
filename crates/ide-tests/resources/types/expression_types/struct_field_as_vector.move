module 0x1::M {
    struct NFT {}
    struct Collection { nfts: vector<NFT> }
    fun m(coll: Collection) {
        (coll.nfts);
      //^ vector<0x1::M::NFT>  
    }
}    