#!/bin/sh

# License: CC0 1.0 Universal
# https://creativecommons.org/publicdomain/zero/1.0/legalcode

set -eufx

D="$(dirname "$0")"

. "$D/common.sh"
. "$D/travis-doc-upload.cfg"

[ "$TRAVIS_BRANCH" = master ] || [ "$TRAVIS_BRANCH" = "doc-test" ]

# FIXME: ssh known hosts handling does not appear to work with travis-osx
[ "$TRAVIS_OS_NAME" != osx ]

[ "$TRAVIS_PULL_REQUEST" = false ]

set +x
eval key=\$encrypted_${SSH_KEY_TRAVIS_ID}_key
eval iv=\$encrypted_${SSH_KEY_TRAVIS_ID}_iv
set -x

# TODO: generalize over other key types (not just rsa)
mkdir -p ~/.ssh
# travis OSX doesn't add these automatically (linux does)
echo >> ~/.ssh/config <<EOF
Host github.com
	StrictHostKeyChecking no
EOF
set +x
openssl aes-256-cbc -K "$key" -iv "$iv" -in "$D/docs_github_id.enc" -out ~/.ssh/id_rsa -d
set -x
chmod -R u=rwX ~/.ssh
# XXX: the above for some reason isn't working.
chmod 600 ~/.ssh/id_rsa
chmod 600 ~/.ssh/config

git clone --branch gh-pages "git@github.com:$DOCS_REPO" deploy_docs || {
	git clone "git@github.com:$DOCS_REPO" deploy_docs
}

cd deploy_docs
git config user.name "doc upload bot"
git config user.email "nobody@example.com"
rm -rf "$PROJECT_NAME"
mkdir -p "$(dirname "$PROJECT_NAME")"
mv ../target/$TARGET/doc "$PROJECT_NAME"

# For each element of $PROJECT_NAME generate an index
# this _must_ be the crate we care about, used to suffix last indexing
# cursor for iteration
crate_name="$(printf "%s" "$PROJECT_BASE" | sed 's/-/_/g')"

gen_commit () {
	curr="$(dirname "$PROJECT_NAME")"
	../"$D"/generate-index.sh "$curr" "$crate_name/index.html"
	if ! [ . = "$curr" ]; then
		curr="$(dirname "$curr")"
		while true; do
			../"$D"/generate-index.sh "$curr"
			if [ . = "$curr" ]; then
				break
			fi
			curr="$(dirname "$curr")"
		done
	fi

	git add -A .
	git commit -qm "doc upload for $PROJECT_NAME ($TRAVIS_REPO_SLUG)"
}

rollback_commit() {
	git reset HEAD^
	# moves HEAD & index, but not workdir

	curr="$(dirname "$PROJECT_NAME")"
	git checkout index.html
	if ! [ . = "$curr" ]; then
		curr="$(dirname "$curr")"
		while true; do
			git checkout index.html
			if [ . = "$curr" ]; then
				break
			fi
			curr="$(dirname "$curr")"
		done
	fi
}

gen_commit

while ! git push -q origin HEAD:refs/heads/gh-pages; do
	rollback_commit
	git pull
	gen_commit
done
