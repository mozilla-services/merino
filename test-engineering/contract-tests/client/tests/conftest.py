# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import json
import os
import pathlib
from functools import lru_cache
from typing import Dict, Set

import pytest
import yaml
from _pytest.fixtures import SubRequest
from requests import Response as RequestsResponse

import kinto
from exceptions import MissingKintoDataFilesError
from kinto import KintoAttachment, KintoEnvironment, KintoRecord
from models import KintoSuggestion, Scenario

REQUIRED_OPTIONS = (
    "scenarios_file",
    "merino_url",
    "kinto_url",
    "kinto_bucket",
    "kinto_collection",
    "kinto_data_dir",
    "kinto_attachments_url",
)


@pytest.fixture(scope="session", name="kinto_environment")
def fixture_kinto_environment(request: SubRequest) -> KintoEnvironment:
    """Return Kinto environment data."""

    return KintoEnvironment(
        api=request.config.option.kinto_url,
        bucket=request.config.option.kinto_bucket,
        collection=request.config.option.kinto_collection,
    )


@pytest.fixture(scope="session", name="kinto_icon_urls")
def fixture_kinto_icon_urls(
    request: SubRequest, kinto_environment: KintoEnvironment
) -> Dict[str, str]:
    """Return a map from suggestion title to icon URL."""

    attachments_url: str = request.config.option.kinto_attachments_url

    @lru_cache(maxsize=None)
    def fetch_icon_url(*, record_id: str) -> str:
        """Fetch the icon URL for the given Kinto record ID from Kinto."""

        response: RequestsResponse = kinto.get_record(kinto_environment, record_id)
        response.raise_for_status()

        icon_location: str = response.json()["data"]["attachment"]["location"]

        return f"{attachments_url}/{icon_location}"

    return {
        suggestion.title: fetch_icon_url(record_id=f"icon-{suggestion.icon}")
        for suggestion in request.config.kinto_suggestions
    }


@pytest.fixture(scope="session", name="kinto_records")
def fixture_kinto_records(request: SubRequest) -> Dict[str, KintoRecord]:
    """Return a map from data file name to suggestion data."""

    kinto_data_dir: str = request.config.option.kinto_data_dir

    # Load Kinto data from the given Kinto data directory
    kinto_records: Dict[str, KintoRecord] = {}
    for data_file in pathlib.Path(kinto_data_dir).glob("*.json"):

        content: bytes = data_file.read_bytes()
        icon_ids: Set[str] = {suggestion["icon"] for suggestion in json.loads(content)}

        kinto_records[data_file.name] = KintoRecord(
            record_id=data_file.stem,
            attachment=KintoAttachment(
                filename=data_file.name,
                filecontent=content,
                mimetype="application/json",
                icon_ids=icon_ids,
            ),
        )

    if not kinto_records:
        raise MissingKintoDataFilesError(kinto_data_dir)

    return kinto_records


def pytest_configure(config):
    """Load data for tests and store it on config."""

    for option_name in REQUIRED_OPTIONS:
        if getattr(config.option, option_name) is None:
            raise pytest.UsageError(f"Required option '{option_name}' is not set.")

    with pathlib.Path(config.option.scenarios_file).open() as f:
        loaded_scenarios = yaml.safe_load(f)

    config.merino_scenarios = [
        Scenario(**scenario) for scenario in loaded_scenarios["scenarios"]
    ]

    kinto_data_dir = pathlib.Path(config.option.kinto_data_dir)

    config.kinto_suggestions = [
        KintoSuggestion(**suggestion_data)
        for data_file in kinto_data_dir.glob("*.json")
        for suggestion_data in json.loads(data_file.read_text())
    ]


def pytest_generate_tests(metafunc):
    """Generate tests from the loaded test scenarios."""

    if "steps" not in metafunc.fixturenames:
        return

    ids = []
    argvalues = []

    for scenario in metafunc.config.merino_scenarios:
        ids.append(scenario.name)
        argvalues.append([scenario.steps])

    metafunc.parametrize(["steps"], argvalues, ids=ids)


def pytest_addoption(parser):
    """Define custom CLI options."""
    parser.addoption(
        "--scenarios_file",
        action="store",
        dest="scenarios_file",
        help="File with test scenarios",
        metavar="SCENARIOS_FILE",
        default=os.environ.get("SCENARIOS_FILE"),
        type=str,
        required=False,
    )

    merino_group = parser.getgroup("merino")
    merino_group.addoption(
        "--merino-url",
        action="store",
        dest="merino_url",
        help="Merino endpoint URL",
        metavar="MERINO_URL",
        default=os.environ.get("MERINO_URL"),
        type=str,
        required=False,
    )

    kinto_group = parser.getgroup("kinto")
    kinto_group.addoption(
        "--kinto-url",
        action="store",
        dest="kinto_url",
        help="Kinto URL",
        metavar="KINTO_URL",
        default=os.environ.get("KINTO_URL"),
        type=str,
        required=False,
    )
    kinto_group.addoption(
        "--kinto-bucket",
        action="store",
        dest="kinto_bucket",
        help="Kinto bucket",
        metavar="KINTO_BUCKET",
        default=os.environ.get("KINTO_BUCKET"),
        type=str,
        required=False,
    )
    kinto_group.addoption(
        "--kinto-collection",
        action="store",
        dest="kinto_collection",
        help="Kinto collection",
        metavar="KINTO_COLLECTION",
        default=os.environ.get("KINTO_COLLECTION"),
        type=str,
        required=False,
    )
    kinto_group.addoption(
        "--kinto-data-dir",
        action="store",
        dest="kinto_data_dir",
        help="Directory containing Kinto data",
        metavar="KINTO_DATA_DIR",
        default=os.environ.get("KINTO_DATA_DIR"),
        type=str,
        required=False,
    )
    kinto_group.addoption(
        "--kinto-attachments-url",
        action="store",
        dest="kinto_attachments_url",
        help="Kinto attachments URL",
        metavar="KINTO_ATTACHMENTS_URL",
        default=os.environ.get("KINTO_ATTACHMENTS_URL"),
        type=str,
        required=False,
    )
