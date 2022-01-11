# Merino Admin

This is a separate site intended to be deployed along side Merino. It manages
dynamic settings that will (some day) allow Merino's behavior to be changed at
runtime.

To use merino-admin, you'll need a Python development environment, and
[Poetry][]. With that ready, you can set up the Django site:

```shell
# From the repository root
$ cd merino-admin
$ poetry shell
$ poetry install
$ ./manage.py migrate
$ ./manage.py createsuperuser
```

and run the admin site:

```shell
$ ./manage.py runserver
```

This will start a development server that reloads on changes to the files. You
can access the editing part of the site at
[localhost:8000/admin][http://localhost:8000/admin].

[poetry]: https://python-poetry.org/
