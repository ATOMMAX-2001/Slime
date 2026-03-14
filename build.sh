echo "Copying files to publish"

cp slimeweb/lib/cli.py publish/python/slimeweb/slimeweb/cli.py
cp slimeweb/lib/slime.py publish/python/slimeweb/slimeweb/slime.py
cp README.md publish/python/slimeweb/slime.py
maturin build --manifest-path web/Cargo.toml  --release
cp web/target/wheels/* publish/python/slimeweb/slimeweb/web/*


echo "unzipping"
unzip publish/python/slimeweb/slimeweb/web/*.whl
cp publish/python/slimeweb/slimeweb/web/web/*.so publish/python/slimeweb/slimeweb/web/
rm -rf publish/python/slimeweb/slimeweb/web/  publish/python/slimeweb/slimeweb/web/*.dist-info


echo "building"
cd publish/python/slimeweb/
python3 -m build
twine upload dist/*
