# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

from pathlib import Path
from time import sleep

import requests
import typer
from kinto import (
    KintoAttachment,
    KintoRecord,
    create_bucket,
    create_collection,
    upload_attachments,
    upload_icons,
)
from requests import HTTPError


def main(
    kinto_url: str = typer.Argument(..., envvar="KINTO_URL"),
    kinto_bucket: str = typer.Argument(..., envvar="KINTO_BUCKET"),
    kinto_collection: str = typer.Argument(..., envvar="KINTO_COLLECTION"),
    kinto_data_dir: Path = typer.Argument(..., envvar="KINTO_DATA_DIR"),
):
    """Run the CLI application."""

    # Load Kinto data from the given Kinto data directory
    kinto_records = [
        KintoRecord(
            record_id=data_file.stem,
            attachment=KintoAttachment(
                filename=data_file.name,
                mimetype="application/json",
                filecontent=data_file.read_bytes(),
            ),
            data_type="offline-expansion-data"
            if "offline" in data_file.stem
            else "data",
        )
        for data_file in kinto_data_dir.glob("*.json")
    ]

    if not kinto_records:
        typer.echo(f"Cannot find Kinto data files in {kinto_data_dir}", err=True)
        raise typer.Exit(code=1)

    # Load unique icon IDs from the JSON files and store them in a set
    icon_ids = {
        suggestion["icon"]
        for record in kinto_records
        for suggestion in record.attachment.json_suggestions
    }

    kinto_api = f"{kinto_url}/v1"

    try:
        create_bucket(api=kinto_api, bucket=kinto_bucket)
        create_collection(
            api=kinto_api,
            bucket=kinto_bucket,
            collection=kinto_collection,
        )
        upload_attachments(
            api=kinto_api,
            bucket=kinto_bucket,
            collection=kinto_collection,
            records=kinto_records,
        )
        upload_icons(
            api=kinto_api,
            bucket=kinto_bucket,
            collection=kinto_collection,
            icon_ids=icon_ids,
        )
    except HTTPError as exc:
        typer.echo(f"An error occured while setting up Kinto: {exc}", err=True)
        raise typer.Exit(code=1)

    timeout: float = 30.0 * 60
    interval: float = 1.0 * 60

    while timeout > 0.0:
        # Retrieve the server info and sleep for the above interval.
        # We do this so that docker-compose does not terminate when the CLI
        # exits if running with --abort-on-container-exit as is the case on CI.

        response = requests.get(f"{kinto_api}/")

        try:
            response.raise_for_status()
        except HTTPError as exc:
            typer.echo(f"An error occured while connecting to Kinto: {exc}", err=True)
            raise typer.Exit(code=1)

        server_info = response.json()
        typer.echo(f"Kinto still up an running: {server_info=}")

        typer.echo(f"Sleeping for {interval} seconds")
        sleep(interval)

        timeout -= interval

    typer.echo("Shutting down")


if __name__ == "__main__":
    typer.run(main)
