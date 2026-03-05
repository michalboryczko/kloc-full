"""Business logic: reference types, handlers, definition builders."""

from .reference_types import (
    CHAINABLE_REFERENCE_TYPES,
    REF_TYPE_PRIORITY,
    infer_reference_type,
    get_reference_type_from_call_kind,
    sort_entries_by_priority,
    sort_entries_by_location,
)
from .handlers import (
    EdgeContext,
    EntryBucket,
    USED_BY_HANDLERS,
)
from .definition import (
    build_definition,
    parse_property_doc,
)

__all__ = [
    "CHAINABLE_REFERENCE_TYPES",
    "REF_TYPE_PRIORITY",
    "infer_reference_type",
    "get_reference_type_from_call_kind",
    "sort_entries_by_priority",
    "sort_entries_by_location",
    "EdgeContext",
    "EntryBucket",
    "USED_BY_HANDLERS",
    "build_definition",
    "parse_property_doc",
]
