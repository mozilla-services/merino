# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import requests
import typer


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
