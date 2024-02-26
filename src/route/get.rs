// SPDX-License-Identifier: MIT

use std::{
    marker::PhantomData,
    net::{Ipv4Addr, Ipv6Addr},
};

use futures::{
    future::{self, Either},
    stream::{StreamExt, TryStream},
    FutureExt,
};

use netlink_packet_core::{NetlinkMessage, NLM_F_DUMP, NLM_F_REQUEST};
use netlink_packet_route::{
    route::{
        RouteAttribute, RouteHeader, RouteMessage, RouteProtocol, RouteScope,
        RouteType,
    },
    AddressFamily, RouteNetlinkMessage,
};

use crate::{try_rtnl, Error, Handle};

pub struct RouteGetRequest<T = ()> {
    handle: Handle,
    message: RouteMessage,
    // There are two ways to retrieve routes: we can either dump them
    // all and filter the result, or if we already know the destination
    // of the route we're looking for, we can just retrieve
    // that one. If `dump` is `true`, all the routes are fetched.
    // Otherwise, only the best route to the destination is fetched.
    dump: bool,
    _phantom: PhantomData<T>,
}

/// Internet Protocol (IP) version.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub enum IpVersion {
    /// IPv4
    V4,
    /// IPv6
    V6,
}

impl IpVersion {
    pub(crate) fn family(self) -> AddressFamily {
        match self {
            IpVersion::V4 => AddressFamily::Inet,
            IpVersion::V6 => AddressFamily::Inet6,
        }
    }
}

impl<T> RouteGetRequest<T> {
    pub(crate) fn new(handle: Handle) -> Self {
        let mut message = RouteMessage::default();

        // As per rtnetlink(7) documentation, setting the following
        // fields to 0 gets us all the routes from all the tables
        //
        // > For RTM_GETROUTE, setting rtm_dst_len and rtm_src_len to 0
        // > means you get all entries for the specified routing table.
        // > For the other fields, except rtm_table and rtm_protocol, 0
        // > is the wildcard.
        message.header.destination_prefix_length = 0;
        message.header.source_prefix_length = 0;
        message.header.scope = RouteScope::Universe;
        message.header.kind = RouteType::Unspec;

        // I don't know if these two fields matter
        message.header.table = RouteHeader::RT_TABLE_UNSPEC;
        message.header.protocol = RouteProtocol::Unspec;

        RouteGetRequest {
            handle,
            message,
            dump: true,
            _phantom: PhantomData,
        }
    }

    /// Sets the output interface index.
    pub fn output_interface(mut self, index: u32) -> Self {
        self.message.attributes.push(RouteAttribute::Oif(index));
        self
    }

    pub fn message_mut(&mut self) -> &mut RouteMessage {
        &mut self.message
    }
}

impl RouteGetRequest<()> {
    pub fn v4(mut self) -> RouteGetRequest<Ipv4Addr> {
        self.message.header.address_family = AddressFamily::Inet;
        RouteGetRequest::<Ipv4Addr> {
            _phantom: PhantomData::<Ipv4Addr>,
            handle: self.handle,
            message: self.message,
            dump: self.dump,
        }
    }

    pub fn v6(mut self) -> RouteGetRequest<Ipv6Addr> {
        self.message.header.address_family = AddressFamily::Inet6;
        RouteGetRequest::<Ipv6Addr> {
            _phantom: PhantomData::<Ipv6Addr>,
            handle: self.handle,
            message: self.message,
            dump: self.dump,
        }
    }
}

impl RouteGetRequest<Ipv4Addr> {
    /// Get the best route to this destination
    pub fn to(mut self, ip: Ipv4Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Destination(ip.into()));
        self.message.header.destination_prefix_length = 32;
        self.dump = false;
        self
    }

    pub fn from(mut self, ip: Ipv6Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Source(ip.into()));
        self.message.header.source_prefix_length = 32;
        self
    }

    pub fn execute(
        self,
    ) -> impl TryStream<
        Ok = RouteMessage,
        Error = Error,
        Item = Result<RouteMessage, Error>,
    > {
        let RouteGetRequest {
            mut handle,
            message,
            dump,
            _phantom,
        } = self;

        let mut req =
            NetlinkMessage::from(RouteNetlinkMessage::GetRoute(message));
        req.header.flags = if dump {
            NLM_F_REQUEST | NLM_F_DUMP
        } else {
            NLM_F_REQUEST
        };

        match handle.request(req) {
            Ok(response) => Either::Left(response.map(move |msg| {
                Ok(try_rtnl!(msg, RouteNetlinkMessage::NewRoute))
            })),
            Err(e) => Either::Right(
                future::err::<RouteMessage, Error>(e).into_stream(),
            ),
        }
    }
}

impl RouteGetRequest<Ipv6Addr> {
    /// Get the best route to this destination
    pub fn to(mut self, ip: Ipv6Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Destination(ip.into()));
        self.message.header.destination_prefix_length = 32;
        self.dump = false;
        self
    }

    pub fn from(mut self, ip: Ipv6Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Source(ip.into()));
        self.message.header.source_prefix_length = 32;
        self
    }

    pub fn execute(
        self,
    ) -> impl TryStream<
        Ok = RouteMessage,
        Error = Error,
        Item = Result<RouteMessage, Error>,
    > {
        let RouteGetRequest {
            mut handle,
            message,
            dump,
            _phantom,
        } = self;

        let mut req =
            NetlinkMessage::from(RouteNetlinkMessage::GetRoute(message));
        req.header.flags = if dump {
            NLM_F_REQUEST | NLM_F_DUMP
        } else {
            NLM_F_REQUEST
        };

        match handle.request(req) {
            Ok(response) => Either::Left(response.map(move |msg| {
                Ok(try_rtnl!(msg, RouteNetlinkMessage::NewRoute))
            })),
            Err(e) => Either::Right(
                future::err::<RouteMessage, Error>(e).into_stream(),
            ),
        }
    }
}
