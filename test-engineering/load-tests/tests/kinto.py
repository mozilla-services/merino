# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import json
from typing import Dict, List

import kinto_http
import requests
from pydantic import BaseModel, Extra


class KintoSuggestion(BaseModel, extra=Extra.ignore):
    """Class that holds information about a Suggestion returned by Kinto."""

    advertiser: str
    title: str
    keywords: List[str]


def download_suggestions(client: kinto_http.Client) -> Dict[int, KintoSuggestion]:
    """Get records, download attachments and return the suggestions."""

    # Retrieve the base_url for attachments
    server_info = client.server_info()
    attachments_base_url = server_info["capabilities"]["attachments"]["base_url"]

    # Only consider "data" records, search for the following code in Merino
    # for record in remote_settings_client.records_of_type("data".to_string())
    data_records = [
        record for record in client.get_records() if record["type"] == "data"
    ]

    # Make use of connection pooling because all requests go to the same host
    requests_session = requests.Session()

    suggestions = {}

    for record in data_records:
        attachment_url = f"{attachments_base_url}{record['attachment']['location']}"

        response = requests_session.get(attachment_url)

        if response.status_code != 200:
            # Ignore unsuccessful requests for now
            continue

        # Each attachment is a list of suggestion objects
        # Each suggestion objects contains a list of keywords
        attachment = json.loads(response.text)

        # Load into pydantic model to discard all fields we don't care about
        suggestions.update(
            {
                suggestion_data["id"]: KintoSuggestion(**suggestion_data)
                for suggestion_data in attachment
            }
        )

    return suggestions
