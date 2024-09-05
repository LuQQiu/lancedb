set -e

RELEASE_TYPE=${1:-"stable"}
BUMP_MINOR=${2:-false}
TAG_PREFIX=${3:-"v"} # Such as "python-v", "java-v"
HEAD_SHA=${4:-$(git rev-parse HEAD)}

readonly SELF_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

PREV_TAG=$(git tag --sort='version:refname' | grep ^$TAG_PREFIX | python $SELF_DIR/semver_sort.py $TAG_PREFIX | tail -n 1)
echo "Found previous tag $PREV_TAG"

bump_java_version() {
  local bump_type=$1

  current_version="${PREV_TAG#$TAG_PREFIX}"

  if [[ "$current_version" == *"-beta."* ]]; then
    # Extract the base version (X.Y.Z) and beta part (N)
    base_version="${current_version%-beta.*}"
    beta_part="${current_version##*-beta.}"
  else
    # The version is stable (X.Y.Z)
    base_version="$current_version"
    beta_part=""
  fi

  major=${base_version%%.*}
  minor=${base_version#*.}
  minor=${minor%%.*}
  patch=${base_version##*.}

  case $bump_type in
  "minor")
    minor=$((minor + 1))
    patch=0
    beta_part="0"
    ;;
  "patch")
    patch=$((patch + 1))
    beta_part="0"
    ;;
  "pre_n")
    if [[ -n "$beta_part" ]]; then
      beta_part=$((beta_part + 1))
    else
      beta_part="0"
    fi
    ;;
  *)
    echo "Invalid bump type specified: ${bump_type}"
    exit 1
    ;;
  esac

  new_version="${major}.${minor}.${patch}"
  if [ "$RELEASE_TYPE" != "stable" ]; then
    new_version="${new_version}-beta.${beta_part}"
  fi
  mvn versions:set versions:commit -DnewVersion="$new_version"

  if [ "$RELEASE_TYPE" != "stable" ]; then
    # Create tag similar to bump-my-version
  fi
}

bump_version() {
  local bump_type=$1
  local bump_args=""

  # Initially, we don't want to tag if we are doing stable, because we will bump
  # again later. See comment at end for why.
  if [ "$RELEASE_TYPE" == "stable" ]; then
    bump_args="--no-tag"
  fi

  bump-my-version bump -vv $bump_args $bump_type

  # The above bump will always bump to a pre-release version. If we are releasing
  # a stable version, bump the pre-release level ("pre_l") to make it stable.
  if [ "$RELEASE_TYPE" == "stable" ]; then
    # X.Y.Z-beta.N -> X.Y.Z
    bump-my-version bump -vv pre_l
  fi
}

# If last is stable and not bumping minor
if [[ $PREV_TAG != *beta* ]]; then
  if [ "$BUMP_MINOR" != "false" ]; then
    # X.Y.Z -> X.(Y+1).0-beta.0
    BUMP_TYPE="minor"
  else
    # X.Y.Z -> X.Y.(Z+1)-beta.0
    BUMP_TYPE="patch"
  fi
else
  if [ "$BUMP_MINOR" != "false" ]; then
    # X.Y.Z-beta.N -> X.(Y+1).0-beta.0
    BUMP_TYPE="minor"
  else
    # X.Y.Z-beta.N -> X.Y.Z-beta.(N+1)
    BUMP_TYPE="pre_n"
  fi
fi

if [ "$TAG_PREFIX" = "java-v" ]; then
  bump_java_version "$BUMP_TYPE"
else
  bump_version "$BUMP_TYPE"
fi

# Validate that we have incremented version appropriately for breaking changes
NEW_TAG=$(git describe --tags --exact-match HEAD)
NEW_VERSION=$(echo $NEW_TAG | sed "s/^$TAG_PREFIX//")
LAST_STABLE_RELEASE=$(git tag --sort='version:refname' | grep ^$TAG_PREFIX | grep -v beta | grep -vF "$NEW_TAG" | python $SELF_DIR/semver_sort.py $TAG_PREFIX | tail -n 1)
LAST_STABLE_VERSION=$(echo $LAST_STABLE_RELEASE | sed "s/^$TAG_PREFIX//")

python $SELF_DIR/check_breaking_changes.py $LAST_STABLE_RELEASE $HEAD_SHA $LAST_STABLE_VERSION $NEW_VERSION
