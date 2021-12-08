FROM python:3.8-slim-buster

LABEL maintainer "Raphael Pierzina <raphael@hackebrot.de>"

ENV PYTHON_VENV=/venv
RUN python -m venv ${PYTHON_VENV}
ENV PATH="${PYTHON_VENV}/bin:${PATH}"

RUN python -m pip install --upgrade pip

COPY requirements.txt /tmp/requirements.txt
RUN python -m pip install -r /tmp/requirements.txt

COPY . /usr/src/cli
WORKDIR /usr/src/cli

CMD [ "python", "main.py" ]
