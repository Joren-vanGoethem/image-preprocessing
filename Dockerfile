FROM rust:1.67

WORKDIR /usr/src/myapp
COPY . .

RUN apt-get -y update
RUN apt-get -y upgrade
RUN apt-get install -y ffmpeg exiftran

RUN cargo install --path .

CMD ["image_preprocessing"]
