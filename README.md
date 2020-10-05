<!--
*** Thanks for checking out this README Template. If you have a suggestion that would
*** make this better, please fork the repo and create a pull request or simply open
*** an issue with the tag "enhancement".
*** Thanks again! Now go create something AMAZING! :D
***
***
***
*** To avoid retyping too much info. Do a search and replace for the following:
*** nicomazz, fastgtfs, nicomazz, nicomazz97+fastgtfs@gmail.com
-->





<!-- PROJECT SHIELDS -->

[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]




<!-- PROJECT LOGO -->
<br />
<p align="center">
  <h3 align="center">fastgtfs</h3>

  <a href="https://github.com/nicomazz/fastgtfs">
   <!-- <img src="images/logo.png" alt="Logo" width="80" height="80">-->
   <img src="https://i.ibb.co/XDqf83x/fastgtfs-cover.png" alt="fastgtfs-cover" border="0">
  </a>


  <p align="center">
    A pure Rust library that provides <a href="https://developers.google.com/transit/gtfs">GTFS</a> parsing, navigation, time table creation and real time network simulation. Already in production with the beta of <a href="https://play.google.com/apps/testing/com.actv.nicomazz.lastjni">this</a> android app. 
    <br />
    <br />
    <br />
    <a href="https://play.google.com/apps/testing/com.actv.nicomazz.lastjni">View Demo</a>
    ·
    <a href="https://github.com/nicomazz/fastgtfs/issues">Report Bug</a>
    ·
    <a href="https://github.com/nicomazz/fastgtfs/issues">Request Feature</a>
  </p>
</p>



<!-- TABLE OF CONTENTS -->
## Table of Contents

* [About the Project](#about-the-project)
* [Usage](#usage)
* [License](#license)
* [Contact](#contact)


<!-- ABOUT THE PROJECT -->
## About The Project


This library is the core of the [Venice Navigation&timetables](https://play.google.com/apps/testing/com.actv.nicomazz.lastjni) app. The aim of the app is to provide services using GTFS data completely **offline**. In this way, tourists without an intenet plan do not have to worry about public transport.
This all was originally written in C++. In order to learn RUST, I decided to write it all again (so, yes, this is my first rust project). It has been cross compiled for Android, and provides several features to the app:

- Navigation from point A to B, with a slightly modified version of Microsoft's [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf) algorithm. This version takes care of real walking path distances, and average person preferences when choosing a solution.

- Creation of timetables for routes with heterogeneous trips. To do that, I do a topological sort on the different trip paths.

- Simulation of the entire network. Given a trip, and a time, it is possible to get the exact position of the bus, by using interpolation. This scales well: in the android app it is possible to see a simulation of the entire venice bus and water-bus network.

- Merge of several datasets into a unique data structure.

- Optimized queries over GTFS dataset:
    - todo: list those queries
    
- Serialization and deserialization of the raw data structure using Google's [flatbuffers](https://google.github.io/flatbuffers/). This feature is used for the Android app. Every time the app is started, it reads the binary data (in the flatbuffer format) directly into the final data structure, avoiding the slow txt parsing. The parsing requires more time, and generates the serialized binary data.

- Walk time calculator. It uses "HERE" APIs to precompute the real walking times between each stop and the 40 nearest ones. This is done with the `walk_distance_calculator` crate. It uses parsed stop positions.

There are a few basic tests, where they are really needed. From the Android app, it seems it really works well!


<!-- USAGE EXAMPLES -->
## Usage

For the usage, refer to the `tests` folder.


<!-- LICENSE -->
## License

Distributed under the GNU General Public License v3.0 License. See `LICENSE` for more information.



<!-- CONTACT -->
## Contact

Nicolo' Mazzucato - [@nicomazz](https://twitter.com/nicomazz)

Project Link: [https://github.com/nicomazz/fastgtfs](https://github.com/nicomazz/fastgtfs)





<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/nicomazz/repo.svg?style=flat-square
[contributors-url]: https://github.com/nicomazz/repo/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/nicomazz/repo.svg?style=flat-square
[forks-url]: https://github.com/nicomazz/repo/network/members
[stars-shield]: https://img.shields.io/github/stars/nicomazz/repo.svg?style=flat-square
[stars-url]: https://github.com/nicomazz/repo/stargazers
[issues-shield]: https://img.shields.io/github/issues/nicomazz/repo.svg?style=flat-square
[issues-url]: https://github.com/nicomazz/repo/issues
[license-shield]: https://img.shields.io/github/license/nicomazz/repo.svg?style=flat-square
[license-url]: https://github.com/nicomazz/repo/blob/master/LICENSE.txt
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=flat-square&logo=linkedin&colorB=555
[linkedin-url]: https://linkedin.com/in/nicomazz
[product-screenshot]: images/screenshot.png