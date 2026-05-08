#! /bin/sh

get_url() {
	case "$1" in
		# Compression
		zlib)      echo "https://www.zlib.net/zlib-1.3.1.tar.gz" ;;
		bzip2)     echo "https://sourceware.org/pub/bzip2/bzip2-1.0.8.tar.gz" ;;
		# Images
		libpng)    echo "https://download.sourceforge.net/libpng/libpng-1.6.53.tar.gz" ;;
		libjpeg)   echo "http://www.ijg.org/files/jpegsrc.v9f.tar.gz" ;;
		openjpeg)  echo "https://github.com/uclouvain/openjpeg/archive/v2.5.4.tar.gz" ;;
		jbig2dec)  echo "https://github.com/ArtifexSoftware/jbig2dec/releases/download/0.20/jbig2dec-0.20.tar.gz" ;;
		# Fonts
		freetype2) echo "https://download.savannah.gnu.org/releases/freetype/freetype-2.14.1.tar.gz" ;;
		harfbuzz)  echo "https://github.com/harfbuzz/harfbuzz/archive/12.3.0.tar.gz" ;;
		# Latin morphological analysis
		collatinus) echo "https://github.com/rvlander/collatinus/archive/refs/heads/no-qt-improve-loading-perf.tar.gz" ;;
		# Documents
		gumbo)     echo "https://github.com/google/gumbo-parser/archive/v0.10.1.tar.gz" ;;
		djvulibre) echo "http://downloads.sourceforge.net/djvu/djvulibre-3.5.29.tar.gz" ;;
		mupdf)     echo "https://casper.mupdf.com/downloads/archive/mupdf-1.27.0-source.tar.gz" ;;
		*) echo "" ;;
	esac
}

all_names="zlib bzip2 libpng libjpeg openjpeg jbig2dec freetype2 harfbuzz collatinus gumbo djvulibre mupdf"

for name in ${@:-${all_names}} ; do
	url=$(get_url "$name")
	if [ ! "$url" ] ; then
		echo "Unknown library: ${name}." 1>&2
		exit 1
	fi
	echo "Downloading ${name}."
	if [ -d "$name" ]; then
		rm -rf "$name"
		mkdir "$name"
	else
		mkdir "$name"
	fi
	wget -q --show-progress -O "${name}.tgz" "$url"
	tar -xz --strip-components 1 -C "$name" -f "${name}.tgz" && rm "${name}.tgz"
done
