# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from typing import Set

import requests
from pydantic import BaseModel
from requests import Response as RequestsResponse


class KintoEnvironment(BaseModel):
    """Class that holds information about Kinto environment variables."""

    api: str
    bucket: str
    collection: str


class KintoAttachment(BaseModel):
    """Class that holds information about an attachment in Kinto."""

    filename: str
    filecontent: bytes
    mimetype: str
    icon_ids: Set[str]


class KintoRecord(BaseModel):
    """Class that holds information about a record in Kinto."""

    record_id: str
    attachment: KintoAttachment


def get_record(environment: KintoEnvironment, record_id: str) -> RequestsResponse:
    """Get attachment information from Kinto for the given record ID."""

    url: str = (
        f"{environment.api}/v1/"
        f"buckets/{environment.bucket}/"
        f"collections/{environment.collection}/"
        f"records/{record_id}"
    )
    return requests.get(url)


def upload_attachment(
    environment: KintoEnvironment, record: KintoRecord, data_type: str
) -> RequestsResponse:
    """Upload attachment to Kinto for the given record."""

    url: str = (
        f"{environment.api}/v1/"
        f"buckets/{environment.bucket}/"
        f"collections/{environment.collection}/"
        f"records/{record.record_id}/"
        f"attachment"
    )
    return requests.post(
        url=url,
        files={
            "attachment": (
                record.attachment.filename,
                record.attachment.filecontent,
                record.attachment.mimetype,
            ),
        },
        data={'data': f'{{"type": "{data_type}"}}'},
    )


def upload_icon(environment: KintoEnvironment, icon_id: str) -> RequestsResponse:
    """Upload icon attachment to Kinto for the given icon ID."""

    url: str = (
        f"{environment.api}/v1/"
        f"buckets/{environment.bucket}/"
        f"collections/{environment.collection}/"
        f"records/icon-{icon_id}/"
        f"attachment"
    )
    return requests.post(
        url=url,
        files={
            "attachment": (
                f"icon-{icon_id}.png",
                f"icon-{icon_id}",
                "image/png",
            ),
        },
        data={'data': '{"type": "icon"}'},
    )
