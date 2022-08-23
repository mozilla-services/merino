# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import re
from time import sleep

import requests
import typer
from requests import HTTPError

from kinto import create_bucket, create_collection

# Pattern to extract data types of Kinto attachment files.
PATTERN_DATA_TYPE = re.compile(r"^(?P<data_type>.*)-\d{2}$")


def main(
    kinto_url: str = typer.Argument(..., envvar="KINTO_URL"),
    kinto_bucket: str = typer.Argument(..., envvar="KINTO_BUCKET"),
    kinto_collection: str = typer.Argument(..., envvar="KINTO_COLLECTION"),
):
    """Run the CLI application."""
    kinto_api = f"{kinto_url}/v1"

    try:
        create_bucket(api=kinto_api, bucket=kinto_bucket)
        create_collection(
            api=kinto_api,
            bucket=kinto_bucket,
            collection=kinto_collection,
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
