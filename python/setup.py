from setuptools import setup, find_packages
from setuptools.command.install import install
from pathlib import Path
import subprocess
import shutil
import os

repo_root = Path(__file__).parent.parent

class CustomInstallCommand(install):
    """Customized setuptools install command to build the wasm module."""

    def run(self):
        # Build the wasm module using Cargo
        subprocess.check_call(
            ['cargo', 'build', '--release', '--target=wasm32-wasi', '--features=bindings'],
            cwd=repo_root
        )
        # Strip the wasm binary
        wasm_file = 'target/wasm32-wasi/release/asciirend.wasm'
        subprocess.check_call(
            ['wasm-strip', wasm_file],
            cwd=repo_root
        )

        # Any additional custom steps can go here, like copying the
        # built WASM file to the appropriate location within the package
        target_dir = os.path.join(self.build_lib, 'asciirend')
        os.makedirs(target_dir, exist_ok=True)
        shutil.copy(os.path.join('..', wasm_file), os.path.join(target_dir, 'wasm.wasm'))

        # Call the standard install command
        install.run(self)

setup(
    name='asciirend',
    version='0.2.1',
    packages=find_packages(),
    install_requires=[
        # list of your package dependencies
        'wasmtime',
    ],
    # Additional metadata
    author='Aurimas Blažulionis',
    author_email='0x60@pm.me',
    description='ascii rendering engine',
    long_description=open('../README.md').read(),
    long_description_content_type='text/markdown',
    url='https://github.com/h33p/asciirend',
    cmdclass={
        'install': CustomInstallCommand,
    },
    classifiers=[
        'Development Status :: 3 - Alpha',
        'Intended Audience :: Developers',
        'License :: OSI Approved :: MIT License',
        'Programming Language :: Python :: 3',
    ],
)
