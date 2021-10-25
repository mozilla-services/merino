# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import json
from itertools import groupby
from operator import itemgetter
from pathlib import Path
from typing import Dict, List, Tuple


def get_query_groups(query_to_id: Dict[str, int]) -> List[Tuple[str, ...]]:
    """Return a list of tuples of queries for the given mapping from query to
    result ID.
    """
    # This assumes a tuple of (query, result ID)
    get_id = itemgetter(1)

    # Do not sort query_to_id by result ID because it's already in an order
    # which is useful for us for generating the search queries
    return [
        tuple(sorted(query for query, _ in group))
        for _, group in groupby(query_to_id.items(), get_id)
    ]


def load_from_file(data_file: Path) -> List[Tuple[str, ...]]:
    """Load the mapping from query to result ID from the given file and return a
    list of query groups.
    """
    with data_file.open(encoding="utf-8") as f:
        data = json.load(f)
    return get_query_groups(data["mapping"])
