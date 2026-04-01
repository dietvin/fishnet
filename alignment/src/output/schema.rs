pub trait OutputSchema: Send {
    const HAS_QUERY_TO_SIGNAL: bool;
    const HAS_REF_TO_SIGNAL: bool;
    const HAS_REF_META: bool;
    const HAS_QUERY_SEQ: bool;
    const HAS_REF_SEQ: bool;
    const HAS_SIGNAL: bool;
}

pub struct QueryBasic;
impl OutputSchema for QueryBasic {
    const HAS_QUERY_TO_SIGNAL: bool = true;
    const HAS_REF_TO_SIGNAL: bool = false;
    const HAS_REF_META: bool = false;
    const HAS_QUERY_SEQ: bool = false;
    const HAS_REF_SEQ: bool = false;
    const HAS_SIGNAL: bool = false;
}

pub struct RefBasic;
impl OutputSchema for RefBasic {
    const HAS_QUERY_TO_SIGNAL: bool = false;
    const HAS_REF_TO_SIGNAL: bool = true;
    const HAS_REF_META: bool = true;
    const HAS_QUERY_SEQ: bool = false;
    const HAS_REF_SEQ: bool = false;
    const HAS_SIGNAL: bool = false;
}

pub struct BothBasic;
impl OutputSchema for BothBasic {
    const HAS_QUERY_TO_SIGNAL: bool = true;
    const HAS_REF_TO_SIGNAL: bool = true;
    const HAS_REF_META: bool = true;
    const HAS_QUERY_SEQ: bool = false;
    const HAS_REF_SEQ: bool = false;
    const HAS_SIGNAL: bool = false;
}

pub struct QueryWithSeq;
impl OutputSchema for QueryWithSeq {
    const HAS_QUERY_TO_SIGNAL: bool = true;
    const HAS_REF_TO_SIGNAL: bool = false;
    const HAS_REF_META: bool = false;
    const HAS_QUERY_SEQ: bool = true;
    const HAS_REF_SEQ: bool = false;
    const HAS_SIGNAL: bool = false;
}

pub struct RefWithSeq;
impl OutputSchema for RefWithSeq {
    const HAS_QUERY_TO_SIGNAL: bool = false;
    const HAS_REF_TO_SIGNAL: bool = true;
    const HAS_REF_META: bool = true;
    const HAS_QUERY_SEQ: bool = false;
    const HAS_REF_SEQ: bool = true;
    const HAS_SIGNAL: bool = false;
}

pub struct BothWithSeq;
impl OutputSchema for BothWithSeq {
    const HAS_QUERY_TO_SIGNAL: bool = true;
    const HAS_REF_TO_SIGNAL: bool = true;
    const HAS_REF_META: bool = true;
    const HAS_QUERY_SEQ: bool = true;
    const HAS_REF_SEQ: bool = true;
    const HAS_SIGNAL: bool = false;
}

pub struct QueryWithSeqAndSig;
impl OutputSchema for QueryWithSeqAndSig {
    const HAS_QUERY_TO_SIGNAL: bool = true;
    const HAS_REF_TO_SIGNAL: bool = false;
    const HAS_REF_META: bool = false;
    const HAS_QUERY_SEQ: bool = true;
    const HAS_REF_SEQ: bool = false;
    const HAS_SIGNAL: bool = true;
}

pub struct RefWithSeqAndSig;
impl OutputSchema for RefWithSeqAndSig {
    const HAS_QUERY_TO_SIGNAL: bool = false;
    const HAS_REF_TO_SIGNAL: bool = true;
    const HAS_REF_META: bool = true;
    const HAS_QUERY_SEQ: bool = false;
    const HAS_REF_SEQ: bool = true;
    const HAS_SIGNAL: bool = true;
}

pub struct BothWithSeqAndSig;
impl OutputSchema for BothWithSeqAndSig {
    const HAS_QUERY_TO_SIGNAL: bool = true;
    const HAS_REF_TO_SIGNAL: bool = true;
    const HAS_REF_META: bool = true;
    const HAS_QUERY_SEQ: bool = true;
    const HAS_REF_SEQ: bool = true;
    const HAS_SIGNAL: bool = true;
}