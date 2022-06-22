# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import json
from dataclasses import dataclass, field
from typing import Dict, List, Set

import requests
import typer


@dataclass
class KintoAttachment:
    """Class that holds information about an attachment in Kinto."""

    filename: str
    mimetype: str
    filecontent: bytes
    json_suggestions: List[Dict] = field(init=False)

    def __post_init__(self):
        """Load the JSON from the file content."""
        self.json_suggestions = json.loads(self.filecontent)


@dataclass
class KintoRecord:
    """Class that holds information about a record in Kinto."""

    record_id: str
    attachment: KintoAttachment
    data_type: str

    def __post_init__(self):
        """Ensure the value of `data_type` is valid"""
        if self.data_type not in ["data", "offline-expansion-data"]:
            raise ValueError(
                f"Invlid data type: {self.data_type},"
                f" should be either 'data' or 'offline-expansion-data'."
            )


def create_bucket(*, api: str, bucket: str) -> None:
    """Create a new bucket in Kinto."""
    typer.echo(f"creating {bucket=}")

    response = requests.post(
        url=f"{api}/buckets",
        json={
            "data": {"id": bucket},
            "permissions": {"read": ["system.Everyone"]},
        },
    )
    response.raise_for_status()


def create_collection(*, api: str, bucket: str, collection: str) -> None:
    """Create a new collection in Kinto."""
    typer.echo(f"creating {collection=} in {bucket=}")

    response = requests.post(
        url=f"{api}/buckets/{bucket}/collections",
        json={
            "data": {"id": collection},
            "permissions": {"read": ["system.Everyone"]},
        },
    )
    response.raise_for_status()


def upload_attachments(
    *,
    api: str,
    bucket: str,
    collection: str,
    records: List[KintoRecord],
) -> None:
    """Upload attachments to Kinto for the given records."""
    records_url = f"{api}/buckets/{bucket}/collections/{collection}/records"

    for record in records:
        typer.echo(f"uploading attachment for {record.record_id=}")

        response = requests.post(
            url=f"{records_url}/{record.record_id}/attachment",
            files={
                "attachment": (
                    record.attachment.filename,
                    record.attachment.filecontent,
                    record.attachment.mimetype,
                ),
                "data": (None, f'{{"type": "{record.data_type}"}}'),
            },
        )
        response.raise_for_status()


def upload_icons(
    *,
    api: str,
    bucket: str,
    collection: str,
    icon_ids: Set[str],
) -> None:
    """Upload icon attachments to Kinto for the given IDs."""
    records_url = f"{api}/buckets/{bucket}/collections/{collection}/records"

    for icon_id in icon_ids:
        typer.echo(f"uploading icon for {icon_id=}")

        response = requests.post(
            url=f"{records_url}/icon-{icon_id}/attachment",
            files={
                "attachment": (
                    f"icon-{icon_id}.png",
                    f"icon-{icon_id}",
                    "image/png",
                ),
                "data": (None, '{"type": "icon"}'),
            },
        )
        response.raise_for_status()
